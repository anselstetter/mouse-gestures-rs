use crate::{
    device::device::{self, Device, EventType, VirtualDevice},
    writer::Writer,
    InputType,
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread, u16,
};

#[derive(Debug)]
struct State {
    pressed_keys: Vec<u16>,
    recorded_keys: Vec<u16>,
}

pub fn start<M, K, WM, W>(
    mouse: M,
    keyboard: K,
    virtual_mouse: WM,
    writer: W,
    types: Vec<InputType>,
) -> io::Result<()>
where
    M: Device + 'static,
    K: Device + 'static,
    WM: VirtualDevice + 'static,
    W: Writer + Send + 'static,
{
    let state = Arc::new(Mutex::new(State {
        pressed_keys: Vec::new(),
        recorded_keys: Vec::new(),
    }));

    let exit = Arc::new(AtomicBool::new(false));
    let writer = Arc::new(Mutex::new(writer));
    let virtual_mouse = Arc::new(Mutex::new(virtual_mouse));

    let keyboard_enabled = types.contains(&InputType::Keyboard) || types.is_empty();
    let mouse_enabled = types.contains(&InputType::Mouse) || types.is_empty();

    let keyboard_thread = spawn_input_thread(
        keyboard_enabled,
        keyboard,
        None::<Arc<Mutex<WM>>>,
        Arc::clone(&state),
        Arc::clone(&exit),
        Arc::clone(&writer),
    );

    let mouse_thread = spawn_input_thread(
        mouse_enabled,
        mouse,
        Some(Arc::clone(&virtual_mouse)),
        Arc::clone(&state),
        Arc::clone(&exit),
        Arc::clone(&writer),
    );

    keyboard_thread
        .join()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Cannot join keyboard thread"))?
        .map_err(|e| e)?;

    mouse_thread
        .join()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Cannot join mouse thread"))?
        .map_err(|e| e)?;

    Ok(())
}

fn spawn_input_thread<D, WD, W>(
    enabled: bool,
    device: D,
    virtual_device: Option<Arc<Mutex<WD>>>,
    state: Arc<Mutex<State>>,
    exit: Arc<AtomicBool>,
    writer: Arc<Mutex<W>>,
) -> thread::JoinHandle<std::io::Result<()>>
where
    D: Device + 'static,
    WD: VirtualDevice + 'static,
    W: Writer + Send + 'static,
{
    if enabled {
        thread::spawn(move || grab(device, virtual_device, &state, &exit, &writer))
    } else {
        thread::spawn(|| Ok(()))
    }
}

fn grab<D, W, WD>(
    mut device: D,
    virtual_device: Option<Arc<Mutex<WD>>>,
    state: &Arc<Mutex<State>>,
    should_exit: &Arc<AtomicBool>,
    writer: &Arc<Mutex<W>>,
) -> io::Result<()>
where
    D: Device,
    W: Writer,
    WD: VirtualDevice,
{
    let state = Arc::clone(state);
    let should_exit_event = Arc::clone(&should_exit);
    let should_exit_check = Arc::clone(&should_exit);

    device.grab_nonblock(
         |event_type| match event_type {
            EventType::Key { code, value, .. } => {
                let mut state = state.lock().unwrap();

                match value {
                    1 => match device::str_from_code(&code) {
                        Some(name) => {
                            if state.recorded_keys.contains(&code) {
                                return false; // already recorded
                            }

                            writer.lock().unwrap().write(&name);

                            state.pressed_keys.push(code);
                            state.recorded_keys.push(code);
                        }
                        None => {
                            println!(
                                "Key not found. This should not happen. Key down: code={}, value={}",
                                code, value
                            );
                        }
                    },
                    0 => should_exit_event.store(true, Ordering::Relaxed),
                    _ => {}
                }

                value == 0
            },
            EventType::Motion { event_type, code, value, .. } => {
                if let Some(d) = &virtual_device {
                    let mut d = d.lock().unwrap();
                    let _ = d.emit(event_type, code, value);
                }
                false
            },
            EventType::Wheel { code, virtual_code, value, .. } => {
                // Prefer low res
                if code == device::REL_WHEEL_HI_RES || code == device::REL_HWHEEL_HI_RES {
                   return false;
                }

                let mut state = state.lock().unwrap();

                match device::str_from_code(&virtual_code) {
                    Some(name) => {
                        // Since mouse wheel events are motion events, 
                        // a second scroll in the same direction acts as a key up.
                        if state.recorded_keys.contains(&virtual_code) {
                            should_exit_event.store(true, Ordering::Relaxed);

                            return false;
                        }

                        writer.lock().unwrap().write(&name);

                        state.pressed_keys.push(virtual_code);
                        state.recorded_keys.push(virtual_code);
                    }
                    None => {
                        println!(
                            "Key not found. This should not happen. Key down: code={}, value={}",
                            virtual_code, value
                        );
                    }
                }
                false
            },
            EventType::Other { event_type, code, value, .. } => {
                if let Some(d) = &virtual_device {
                    let mut d = d.lock().unwrap();
                    let _ = d.emit(event_type, code, value);
                }
                false
            },
        },
        move || should_exit_check.load(Ordering::Relaxed),
        5,
    )
}

#[cfg(test)]
#[path = "./__keys_test.rs"]
mod test;
