use crate::{
    config::{config::Config, window_provider::WindowProvider},
    device::device::{Device, EventType, VirtualDevice},
    handler::handler::Handler,
    recognizer::recognizer::Recognizer,
};
use std::io::Result;

pub struct DispatcherBuilder<R, VM, VK, WP>
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    config: Config,
    window_provider: WP,
    virtual_mouse: VM,
    virtual_keyboard: VK,
    handlers: Vec<Box<dyn Handler<R, VM, VK, WP>>>,
}

impl<R, VM, VK, WP> DispatcherBuilder<R, VM, VK, WP>
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    pub fn new(
        config: Config,
        window_provider: WP,
        virtual_mouse: VM,
        virtual_keyboard: VK,
    ) -> Self {
        Self {
            config: config,
            window_provider: window_provider,
            virtual_mouse: virtual_mouse,
            virtual_keyboard: virtual_keyboard,
            handlers: Vec::new(),
        }
    }

    pub fn with_handler(mut self, handler: Box<dyn Handler<R, VM, VK, WP>>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn build(self) -> Dispatcher<R, VM, VK, WP> {
        Dispatcher {
            config: self.config,
            window_provider: self.window_provider,
            virtual_mouse: self.virtual_mouse,
            virtual_keyboard: self.virtual_keyboard,
            handlers: self.handlers,
        }
    }
}

pub struct Dispatcher<R, VM, VK, WP>
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    config: Config,
    window_provider: WP,
    virtual_mouse: VM,
    virtual_keyboard: VK,
    handlers: Vec<Box<dyn Handler<R, VM, VK, WP>>>,
}

impl<R, VM, VK, WP> Dispatcher<R, VM, VK, WP>
where
    R: Recognizer,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    pub fn builder(
        config: Config,
        window_provider: WP,
        virtual_mouse: VM,
        virtual_keyboard: VK,
    ) -> DispatcherBuilder<R, VM, VK, WP> {
        DispatcherBuilder::new(config, window_provider, virtual_mouse, virtual_keyboard)
    }

    fn on_motion(&mut self, event_type: u16, code: u16, value: i32, x: i32, y: i32) {
        for handler in &mut self.handlers {
            handler.on_motion(x, y);
        }

        self.virtual_mouse.emit(event_type, code, value).unwrap();
    }

    fn on_key(&mut self, event_type: u16, code: u16, value: i32) {
        if self.handlers.iter_mut().any(|handler| {
            handler.on_key(
                &self.config,
                &self.window_provider,
                &mut self.virtual_mouse,
                &mut self.virtual_keyboard,
                code,
                value,
            )
        }) {
            return;
        }

        self.virtual_mouse.emit(event_type, code, value).unwrap();
    }

    fn on_wheel(&mut self, event_type: u16, code: u16, virtual_code: u16, value: i32) {
        if self
            .handlers
            .iter_mut()
            .any(|handler| handler.on_wheel(code, virtual_code, value))
        {
            return;
        }

        self.virtual_mouse.emit(event_type, code, value).unwrap();
    }

    fn on_event(&mut self, event_type: u16, code: u16, value: i32) {
        self.virtual_mouse.emit(event_type, code, value).unwrap();
    }

    pub fn start<M>(&mut self, mut mouse: M) -> Result<()>
    where
        M: Device,
    {
        mouse.grab(|kind| {
            match kind {
                EventType::Motion {
                    event_type,
                    code,
                    value,
                    x,
                    y,
                } => self.on_motion(event_type, code, value, x, y),
                EventType::Key {
                    event_type,
                    code,
                    value,
                } => self.on_key(event_type, code, value),
                EventType::Wheel {
                    event_type,
                    code,
                    virtual_code,
                    value,
                } => self.on_wheel(event_type, code, virtual_code, value),
                EventType::Other {
                    event_type,
                    code,
                    value,
                } => self.on_event(event_type, code, value),
            };
            return false;
        })
    }
}
