use crate::{
    config::{config::Config, window_provider::WindowProvider},
    device::device::{self, VirtualDevice},
    handler::handler::Handler,
    info,
    recognizer::recognizer::Recognizer,
};
use std::{thread, time::Duration};

pub struct KeyHandler {}

impl KeyHandler {
    pub fn new() -> Self {
        Self {}
    }

    fn send_event(
        &mut self,
        virtual_mouse: &mut impl VirtualDevice,
        virtual_keyboard: &mut impl VirtualDevice,
        event_type: u16,
        code: u16,
        value: i32,
    ) {
        if virtual_keyboard.has_capabilities(event_type, code) {
            let _ = virtual_keyboard.emit(event_type, code, value);
        }

        if virtual_mouse.has_capabilities(event_type, code) {
            let _ = virtual_mouse.emit(event_type, code, value);
        }
    }

    fn handle(
        &mut self,
        config: &Config,
        virtual_mouse: &mut impl VirtualDevice,
        virtual_keyboard: &mut impl VirtualDevice,
        code: u16,
        value: i32,
    ) -> bool {
        let key = match device::str_from_code(&code) {
            Some(k) => k,
            None => return false,
        };

        let binds = match config.binds.get(key) {
            Some(b) => b.clone(),
            None => return false,
        };

        info::binds(&binds, value);

        for (i, key_str) in binds.iter().enumerate() {
            if let Some((event_type, code, value)) = device::values_from_str(key_str, value) {
                if i > 0 {
                    thread::sleep(Duration::from_millis(8));
                }
                self.send_event(virtual_mouse, virtual_keyboard, event_type, code, value);
            }
        }

        true
    }
}

impl<R, VM, VK, WP> Handler<R, VM, VK, WP> for KeyHandler
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    fn on_motion(&mut self, _: i32, _: i32) {}

    fn on_wheel(&mut self, _: u16, _: u16, _: i32) -> bool {
        false
    }

    fn on_key(
        &mut self,
        config: &Config,
        _: &WP,
        virtual_mouse: &mut VM,
        virtual_keyboard: &mut VK,
        code: u16,
        value: i32,
    ) -> bool {
        self.handle(config, virtual_mouse, virtual_keyboard, code, value)
    }
}
