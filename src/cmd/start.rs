use crate::{
    config::{config::Config, window_provider::WindowProvider},
    device::device::{Device, VirtualDevice},
    dispatcher::dispatcher::Dispatcher,
    handler::{executor::Executor, gesture_handler::GestureHandler, key_handler::KeyHandler},
    info,
    recognizer::recognizer::Recognizer,
};
use std::io::Result;

pub fn start<M, VM, VK, R, E, WP>(
    mouse: M,
    virtual_mouse: VM,
    virtual_keyboard: VK,
    recognizer: R,
    executor: E,
    config: Config,
    window_provider: WP,
) -> Result<()>
where
    M: Device,
    VM: VirtualDevice,
    VK: VirtualDevice,
    R: Recognizer + 'static,
    E: Executor + 'static,
    WP: WindowProvider,
{
    info::window_provider(&window_provider);

    let gesture_handler = GestureHandler::new(recognizer, executor, 10);
    let key_handler = KeyHandler::new();

    let mut dispatcher =
        Dispatcher::builder(config, window_provider, virtual_mouse, virtual_keyboard)
            .with_handler(Box::new(gesture_handler))
            .with_handler(Box::new(key_handler))
            .build();

    dispatcher.start(mouse)
}

#[cfg(test)]
#[path = "./__start_test.rs"]
mod test;
