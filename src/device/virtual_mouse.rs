use crate::device::device::{VirtualDevice, EVENT_TYPE_KEY, EVENT_TYPE_RELATIVE};
use evdev::{
    AttributeSet, BusType, InputEvent, InputId, KeyCode, RelativeAxisCode, uinput::VirtualDevice as UInputVirtualDevice
};
use std::{collections::HashSet, io::Result};
use std::ffi::CStr;

pub struct VirtualMouse {
    device: UInputVirtualDevice,
    capabilities: HashSet<(u16, u16)>,
}

impl VirtualMouse {
    pub fn new() -> Result<Self> {
        let key_range = 0x100..=0x151;

        let key_set = key_range
            .clone()
            .map(KeyCode::new)
            .collect::<AttributeSet<KeyCode>>();

        let rel_axes = [
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_Z,
            RelativeAxisCode::REL_RX,
            RelativeAxisCode::REL_RY,
            RelativeAxisCode::REL_RZ,
            RelativeAxisCode::REL_HWHEEL,
            RelativeAxisCode::REL_DIAL,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_MISC,
            RelativeAxisCode::REL_RESERVED,
            RelativeAxisCode::REL_WHEEL_HI_RES,
            RelativeAxisCode::REL_HWHEEL_HI_RES,
        ];

        let capabilities = rel_axes
            .iter()
            .map(|axis| (EVENT_TYPE_RELATIVE, axis.0))
            .chain(key_range.map(|code| (EVENT_TYPE_KEY, code)))
            .collect::<HashSet<_>>();

        let axix_set = AttributeSet::from_iter(&rel_axes);

        let device = UInputVirtualDevice::builder()?
            .input_id(InputId::new(BusType(0x03), 0x046d, 0xc534, 0x0001))
            .name("Virtual Mouse")
            .with_keys(&key_set)?
            .with_relative_axes(&axix_set)?
            .build()?;

        Ok(Self {
            device: device,
            capabilities: capabilities,
        })
    }
}

impl VirtualDevice for VirtualMouse {
    fn emit(&mut self, event_type: u16, code: u16, value: i32) -> Result<()> {
        let event = InputEvent::new(event_type, code, value);

        self.device.emit(&[event])
    }

    fn has_capabilities(&self, event_type: u16, code: u16) -> bool {
        self.capabilities.contains(&(event_type, code))
    }
}
