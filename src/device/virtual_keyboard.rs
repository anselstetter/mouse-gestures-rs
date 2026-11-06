use crate::device::device::{VirtualDevice, EVENT_TYPE_KEY};
use evdev::{uinput::VirtualDevice as UInputVirtualDevice, AttributeSet, InputEvent, KeyCode};
use std::{collections::HashSet, io::Result};

pub struct VirtualKeyboard {
    device: UInputVirtualDevice,
    capabilities: HashSet<(u16, u16)>,
}

impl VirtualKeyboard {
    pub fn new() -> Result<Self> {
        let key_range = 0x1..=0xF8;

        let key_set = key_range
            .clone()
            .map(KeyCode::new)
            .collect::<AttributeSet<KeyCode>>();

        let capabilities = key_range
            .map(|code| (EVENT_TYPE_KEY, code))
            .collect::<HashSet<_>>();

        let device = UInputVirtualDevice::builder()?
            .name("Virtual Keyboard")
            .with_keys(&key_set)?
            .build()?;

        Ok(Self {
            device: device,
            capabilities: capabilities,
        })
    }
}

impl VirtualDevice for VirtualKeyboard {
    fn emit(&mut self, event_type: u16, code: u16, value: i32) -> Result<()> {
        let event = InputEvent::new(event_type, code, value);

        self.device.emit(&[event])
    }

    fn has_capabilities(&self, event_type: u16, code: u16) -> bool {
        self.capabilities.contains(&(event_type, code))
    }
}
