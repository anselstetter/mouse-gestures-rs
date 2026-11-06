use crate::{
    config::config::{normalize, Point},
    device::device::{Device, EventType, VirtualDevice},
    writer::Writer,
};
use std::{
    io::Result,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub fn start<M, VM, W>(mut mouse: M, mut virtual_mouse: VM, writer: W) -> Result<()>
where
    M: Device,
    VM: VirtualDevice,
    W: Writer,
{
    let mut trail = Vec::new();
    let mut recording = false;

    let expected_code = 272; // LEFT_BTN

    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_event = Arc::clone(&should_exit);
    let should_exit_check = Arc::clone(&should_exit);

    mouse.grab_nonblock(
        |event| {
            match event {
                EventType::Motion {
                    event_type,
                    code,
                    value,
                    x,
                    y,
                } => {
                    if recording {
                        trail.push(Point { x, y });
                    }
                    let _ = virtual_mouse.emit(event_type, code, value);
                    false
                }
                EventType::Key { code, value, .. } if code == expected_code => {
                    match value {
                        1 => recording = true,  // Key down
                        0 => recording = false, // Key up
                        _ => {}
                    }
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

    let normalized = normalize(&trail, false);
    let json = serde_json::to_string(&normalized)?;

    writer.write(&json);

    Ok(())
}

#[cfg(test)]
#[path = "./__trail_test.rs"]
mod test;
