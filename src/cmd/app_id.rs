use crate::{
    config::window_provider::WindowProvider,
    device::device::{Device, EventType, VirtualDevice},
    writer::Writer,
};
use std::{
    io::{self, Error, Result},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub fn start<M, VM, WP, W>(
    mut mouse: M,
    mut virtual_mouse: VM,
    window_provider: WP,
    writer: W,
) -> Result<()>
where
    M: Device,
    VM: VirtualDevice,
    WP: WindowProvider,
    W: Writer,
{
    if !window_provider.is_valid() {
        return Err(Error::new(
            io::ErrorKind::NotFound,
            "No window provider found",
        ));
    }

    let expected_code = 272; // LEFT_BTN

    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_event = Arc::clone(&should_exit);
    let should_exit_check = Arc::clone(&should_exit);

    let mut application_id = None;

    mouse.grab_nonblock(
        |event| {
            match event {
                EventType::Motion {
                    event_type,
                    code,
                    value,
                    ..
                } => {
                    let _ = virtual_mouse.emit(event_type, code, value);
                    false
                }
                EventType::Key { code, value, .. } if code == expected_code => {
                    application_id = window_provider.get_app_id();

                    should_exit_event.store(value == 0, Ordering::Relaxed);
                    value == 0 // Quit if key is released
                }
                EventType::Key {
                    event_type,
                    code,
                    value,
                } => {
                    let _ = virtual_mouse.emit(event_type, code, value);
                    false
                }
                EventType::Wheel {
                    event_type,
                    code,
                    value,
                    ..
                } => {
                    let _ = virtual_mouse.emit(event_type, code, value);
                    false
                }
                EventType::Other {
                    event_type,
                    code,
                    value,
                } => {
                    let _ = virtual_mouse.emit(event_type, code, value);
                    false
                }
            }
        },
        move || should_exit_check.load(Ordering::Relaxed),
        2,
    )?;

    let id = application_id.unwrap_or_default();
    writer.write(&id);

    Ok(())
}

#[cfg(test)]
#[path = "./__app_id_test.rs"]
mod test;
