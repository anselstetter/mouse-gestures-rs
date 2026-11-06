use crate::device::device::{self, Device, Drain, EventType};
use crate::info;
use evdev::{Device as EvDevice, InputEvent, RelativeAxisCode};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use nix::sys::epoll;
use std::ffi::OsStr;
use std::io::Result;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::Duration;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
};
use udev::{Enumerator, EventType as UdevEventType, MonitorBuilder};

type DevicesMap = Arc<Mutex<HashMap<PathBuf, (DeviceInfo, thread::JoinHandle<Result<()>>)>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Mouse,
    Keyboard,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub name: String,
    pub device_type: DeviceType,
}

#[derive(Debug)]
pub struct InputDevice {
    device_info: DeviceInfo,
    path: PathBuf,
    device: EvDevice,
    last_x: i32,
    last_y: i32,
}

impl InputDevice {
    pub fn new_from_path(path: &Path, device_type: DeviceType) -> Result<Self> {
        let device = EvDevice::open(path)?;
        let name = device.name().unwrap_or("Unnamed Device").to_string();

        let device_info = DeviceInfo {
            path: path.to_string_lossy().into(),
            name: name,
            device_type,
        };

        Ok(Self {
            device_info,
            path: path.to_path_buf(),
            device,
            last_x: 0,
            last_y: 0,
        })
    }
}

pub struct InputManager {
    device_type: DeviceType,
}

impl InputManager {
    pub fn new(device_type: DeviceType) -> Result<Self> {
        Ok(Self { device_type })
    }
}

impl Device for InputManager {
    fn grab<F>(&mut self, mut on_event: F) -> Result<()>
    where
        F: FnMut(EventType) -> bool,
    {
        let (tx, rx) = mpsc::channel();
        let devices = Arc::new(Mutex::new(HashMap::new()));
        let device_type = self.device_type.clone();
        let tx_clone = tx.clone();
        let devices_clone = Arc::clone(&devices);

        thread::spawn(move || {
            let _ = udev_hotplug_loop(
                device_type,
                tx_clone,
                devices_clone,
                None::<(Arc<fn() -> bool>, u16)>,
            );
        });

        for input in open_devices(self.device_type)? {
            let path_clone = input.path.clone();
            let device_info = input.device_info.clone();
            let handle = spawn_listener_thread(input, tx.clone(), None::<(Arc<fn() -> bool>, u16)>);

            devices
                .lock()
                .unwrap()
                .insert(path_clone, (device_info, handle));
        }

        for ev in rx {
            let Some(ev) = ev else {
                break;
            };
            if on_event(ev) {
                break;
            }
        }

        for handle in devices.lock().unwrap().drain().map(|(_, (_, h))| h) {
            let _ = handle.join();
        }

        Ok(())
    }

    fn grab_nonblock<F, C>(
        &mut self,
        mut on_event: F,
        should_quit: C,
        poll_timeout_millis: u16,
    ) -> Result<()>
    where
        F: FnMut(EventType) -> bool,
        C: Fn() -> bool + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let devices = Arc::new(Mutex::new(HashMap::new()));
        let quit = Arc::new(should_quit);
        let device_type = self.device_type.clone();

        let tx_clone = tx.clone();
        let devices_clone = Arc::clone(&devices);
        let quit_clone = Arc::clone(&quit);

        thread::spawn(move || {
            let _ = udev_hotplug_loop(
                device_type,
                tx_clone,
                devices_clone,
                Some((quit_clone, poll_timeout_millis)),
            );
        });

        for input in open_devices(self.device_type)? {
            let path_clone = input.path.clone();
            let tx_clone = tx.clone();
            let quit_clone = Arc::clone(&quit);
            let device_info = input.device_info.clone();

            let handle =
                spawn_listener_thread(input, tx_clone, Some((quit_clone, poll_timeout_millis)));

            devices
                .lock()
                .unwrap()
                .insert(path_clone, (device_info, handle));
        }

        for ev in rx {
            let Some(ev) = ev else {
                break;
            };
            if on_event(ev) {
                break;
            }
        }

        for handle in devices.lock().unwrap().drain().map(|(_, (_, h))| h) {
            let _ = handle.join();
        }

        Ok(())
    }
}

fn open_devices(device_type: DeviceType) -> Result<Vec<InputDevice>> {
    let mut enumerator = Enumerator::new()?;

    let (property, value) = match device_type {
        DeviceType::Mouse => ("ID_INPUT_MOUSE", "1"),
        DeviceType::Keyboard => ("ID_INPUT_KEYBOARD", "1"),
    };
    enumerator.match_subsystem("input")?;
    enumerator.match_property(property, value)?;

    let devices = enumerator
        .scan_devices()?
        .filter(|device| device.property_value("ID_BUS").is_some())
        .filter_map(|device| {
            device.devnode().and_then(|devnode| {
                InputDevice::new_from_path(&devnode.to_path_buf(), device_type).ok()
            })
        })
        .collect::<Vec<_>>();

    let device_info = devices
        .iter()
        .map(|device| device.device_info.clone())
        .collect::<Vec<_>>();

    info::devices(&device_type, &device_info);

    Ok(devices)
}

fn spawn_listener_thread<F>(
    input: InputDevice,
    tx: mpsc::Sender<Option<EventType>>,
    opts: Option<(Arc<F>, u16)>,
) -> thread::JoinHandle<Result<()>>
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    thread::spawn(move || {
        if let Some((quit, timeout)) = opts {
            input_loop_nonblock(input, tx, move || quit(), timeout)
        } else {
            input_loop(input, tx)
        }
    })
}

fn input_loop(mut input: InputDevice, tx: mpsc::Sender<Option<EventType>>) -> Result<()> {
    input.device.drain_events()?;
    input.device.grab()?;

    'event_loop: loop {
        for ev in input.device.fetch_events()? {
            let device_type = input.device_info.device_type;
            let event_kind = into_event(&ev, &device_type, &mut input.last_x, &mut input.last_y);

            if tx.send(Some(event_kind)).is_err() {
                break 'event_loop;
            }
        }
    }

    input.device.ungrab()
}

fn input_loop_nonblock<F>(
    mut input: InputDevice,
    tx: mpsc::Sender<Option<EventType>>,
    should_quit: F,
    poll_timeout_millis: u16,
) -> Result<()>
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    input.device.drain_events()?;
    input.device.grab()?;
    input.device.set_nonblocking(true)?;

    let epoll = epoll::Epoll::new(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
    let event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, 0);
    epoll.add(&input.device, event)?;

    let mut events = [epoll::EpollEvent::empty(); 2];
    let mut last_x = 0;
    let mut last_y = 0;

    'event_loop: loop {
        match input.device.fetch_events() {
            Ok(iterator) => {
                for ev in iterator {
                    let device_type = input.device_info.device_type;
                    let event_kind = into_event(&ev, &device_type, &mut last_x, &mut last_y);

                    if tx.send(Some(event_kind)).is_err() {
                        break 'event_loop;
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if should_quit() {
                    break;
                }
                epoll.wait(&mut events, poll_timeout_millis)?;
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    // Since the corresponding receiver in the main thread will block,
    // it will stop by receiving None.
    let _ = tx.send(None);

    input.device.ungrab()
}

fn udev_hotplug_loop<F>(
    device_type: DeviceType,
    tx: mpsc::Sender<Option<EventType>>,
    devices: DevicesMap,
    opts: Option<(Arc<F>, u16)>,
) -> Result<()>
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    let monitor = MonitorBuilder::new()?.match_subsystem("input")?.listen()?;

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);

    let fd = monitor.as_raw_fd();
    let mut source_fd = SourceFd(&fd);

    poll.registry()
        .register(&mut source_fd, Token(0), Interest::READABLE)?;

    let quit_fn = opts.as_ref().map(|(quit, _)| Arc::clone(quit));

    loop {
        if quit_fn.as_ref().map_or(false, |q| q()) {
            break;
        }

        poll.poll(&mut events, Some(Duration::from_millis(100)))?;

        for event in &events {
            if event.token() != Token(0) || !event.is_readable() {
                continue;
            }

            for e in monitor.iter() {
                let Some(devnode) = e.devnode() else {
                    continue;
                };
                let dev_path = devnode.to_path_buf();

                let is_relevant = match device_type {
                    DeviceType::Mouse => {
                        e.property_value("ID_INPUT_MOUSE") == Some(OsStr::new("1"))
                    }
                    DeviceType::Keyboard => {
                        e.property_value("ID_INPUT_KEYBOARD") == Some(OsStr::new("1"))
                    }
                };

                if !is_relevant || e.device().property_value("ID_BUS").is_none() {
                    continue;
                }

                match e.event_type() {
                    UdevEventType::Add => {
                        if let Ok(input) = InputDevice::new_from_path(&dev_path, device_type) {
                            info::added(&input.device_info);

                            let device_info = input.device_info.clone();
                            let handle = spawn_listener_thread(
                                input,
                                tx.clone(),
                                opts.as_ref().map(|(quit, t)| (Arc::clone(quit), *t)),
                            );

                            devices
                                .lock()
                                .unwrap()
                                .insert(dev_path, (device_info, handle));
                        }
                    }
                    UdevEventType::Remove => {
                        if let Some((device_info, handle)) =
                            devices.lock().unwrap().remove(&dev_path)
                        {
                            info::removed(&device_info);

                            let _ = handle.join();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn into_event(
    ev: &InputEvent,
    device_type: &DeviceType,
    last_x: &mut i32,
    last_y: &mut i32,
) -> EventType {
    match device_type {
        DeviceType::Mouse => into_mouse_event(ev, last_x, last_y),
        DeviceType::Keyboard => into_keyboard_event(ev),
    }
}

fn into_mouse_event(ev: &InputEvent, last_x: &mut i32, last_y: &mut i32) -> EventType {
    let event_type = ev.event_type();
    let code = ev.code();
    let value = ev.value();

    // println!("{:?} {} {}", event_type, code, value);

    match event_type {
        evdev::EventType::KEY => EventType::Key {
            event_type: event_type.0,
            code,
            value,
        },
        evdev::EventType::RELATIVE if code == RelativeAxisCode::REL_X.0 => {
            *last_x += value;
            EventType::Motion {
                event_type: event_type.0,
                code,
                value,
                x: *last_x,
                y: *last_y,
            }
        }
        evdev::EventType::RELATIVE if code == RelativeAxisCode::REL_Y.0 => {
            *last_y += value;
            EventType::Motion {
                event_type: event_type.0,
                code,
                value,
                x: *last_x,
                y: *last_y,
            }
        }
        evdev::EventType::RELATIVE
            if code == RelativeAxisCode::REL_WHEEL.0
                || code == RelativeAxisCode::REL_WHEEL_HI_RES.0 =>
        {
            let virtual_code = if value > 0 {
                device::code_from_str("BTN_WHEEL_UP")
            } else {
                device::code_from_str("BTN_WHEEL_DOWN")
            }
            .unwrap();

            EventType::Wheel {
                event_type: event_type.0,
                code,
                virtual_code,
                value,
            }
        }
        evdev::EventType::RELATIVE
            if code == RelativeAxisCode::REL_HWHEEL.0
                || code == RelativeAxisCode::REL_HWHEEL_HI_RES.0 =>
        {
            let virtual_code = if value > 0 {
                device::code_from_str("BTN_WHEEL_RIGHT")
            } else {
                device::code_from_str("BTN_WHEEL_LEFT")
            }
            .unwrap();

            EventType::Wheel {
                event_type: event_type.0,
                code,
                virtual_code,
                value,
            }
        }
        _ => EventType::Other {
            event_type: event_type.0,
            code,
            value,
        },
    }
}

fn into_keyboard_event(ev: &InputEvent) -> EventType {
    let (event_type, code, value) = (ev.event_type(), ev.code(), ev.value());
    match event_type {
        evdev::EventType::KEY => EventType::Key {
            event_type: event_type.0,
            code,
            value,
        },
        _ => EventType::Other {
            event_type: event_type.0,
            code,
            value,
        },
    }
}
