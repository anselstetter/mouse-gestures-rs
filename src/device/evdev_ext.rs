use crate::device::device::Drain;
use evdev::Device;
use std::io::Result;

impl Drain for Device {
    fn drain_events(&mut self) -> Result<()> {
        self.set_nonblocking(true)?;

        loop {
            match self.fetch_events() {
                Ok(iterator) => {
                    for _ev in iterator {
                        // drain events
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("{e}");
                    break;
                }
            }
        }

        self.set_nonblocking(false)
    }
}
