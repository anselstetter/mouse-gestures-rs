use crate::{
    config::{config::Config, window_provider::WindowProvider},
    device::device::VirtualDevice,
    recognizer::recognizer::Recognizer,
};

pub trait Handler<R, VM, VK, WP>
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    fn on_motion(&mut self, x: i32, y: i32);

    fn on_wheel(&mut self, code: u16, virtual_code: u16, value: i32) -> bool;

    fn on_key(
        &mut self,
        config: &Config,
        window_provider: &WP,
        virtual_mouse: &mut VM,
        virtual_keyboard: &mut VK,
        code: u16,
        value: i32,
    ) -> bool;
}
