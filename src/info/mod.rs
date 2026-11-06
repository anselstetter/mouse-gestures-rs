use std::io;

use crate::{
    config::window_provider::WindowProvider,
    device::input_manager::{DeviceInfo, DeviceType},
    recognizer::recognizer::Candidate,
};

#[cfg(not(test))]
fn print_stderr(args: std::fmt::Arguments) {
    eprintln!("{}", args);
}

#[cfg(test)]
fn print_stderr(_args: std::fmt::Arguments) {
    // do nothing
}

macro_rules! infoln {
    () => {
        infoln!("")
    };
    ($($arg:tt)*) => {
        print_stderr(format_args!($($arg)*))
    };
}

pub fn window_provider(window_provider: &impl WindowProvider) {
    infoln!("{}", dim("Display protocol:"));
    infoln!(
        "{}",
        bold(&format!("{}", window_provider.get_display_protocol()))
    );
    infoln!();

    match window_provider.get_wm() {
        Some(wm) => {
            infoln!("{}", dim("Window manager:"));
            infoln!("{}", bold(&wm));
        }
        None => {
            infoln!(
                "{}",
                bold("No supported wm detected. Only global actions are available")
            );
        }
    };

    infoln!()
}

pub fn udev_event(event: &str, device_info: &DeviceInfo) {
    match device_info.device_type {
        DeviceType::Mouse => infoln!("{}:", dim(&format!("Mouse {}", event))),
        DeviceType::Keyboard => infoln!("{}:", dim(&format!("Keyboard {}", event))),
    }
    infoln!("{} {}", bold(&device_info.name), dim(&device_info.path));
    infoln!()
}

pub fn added(device_info: &DeviceInfo) {
    udev_event("added", device_info);
}

pub fn removed(device_info: &DeviceInfo) {
    udev_event("removed", device_info);
}

pub fn devices(device_type: &DeviceType, devices: &Vec<DeviceInfo>) {
    match device_type {
        DeviceType::Mouse => infoln!("{}:", dim("Mice")),
        DeviceType::Keyboard => infoln!("{}:", dim("Keyboard")),
    }

    devices.iter().for_each(|device| {
        infoln!("{} {}", bold(&device.name), dim(&device.path));
    });

    infoln!()
}

pub fn error(error: &io::Error) {
    infoln!("{}", bold(&format!("{:?}", error)));
}

pub fn propabilities(candidates: &[Candidate]) {
    candidates.iter().enumerate().for_each(|(i, candidate)| {
        let name = &candidate.binding.name;
        let probability = format!("{:.6}", (100.0 - 100.0 * candidate.diff.powi(2)).max(0.0));
        let diff = format!("{:.6}", candidate.diff);

        if i == 0 {
            infoln!("{}\t{}\t{}", bold(&name), bold(&probability), bold(&diff));
        } else {
            infoln!("{}\t{}\t{}", dim(&name), dim(&probability), dim(&diff));
        }
    });

    infoln!()
}

pub fn keys(keys: &Vec<String>) {
    let as_str = keys.join(" + ");

    infoln!("{}", dim("Keys:"));
    infoln!("{}", bold(&as_str));
    infoln!()
}

pub fn command(command: &str) {
    infoln!("{}", dim("Command:"));
    infoln!("{}", bold(command));
    infoln!()
}

pub fn app_id(app_id: &str) {
    infoln!("{}", dim("App ID:"));
    infoln!("{}", bold(app_id));
    infoln!()
}

pub fn modifiers(modifiers: &Vec<&str>) {
    let as_str = modifiers.join(" + ");

    infoln!("{}", dim("Modifiers:"));
    infoln!("{}", bold(&as_str));
    infoln!()
}

pub fn binds(binds: &Vec<String>, value: i32) {
    let as_str = binds.join(" + ");
    let action = if value == 1 { "down" } else { "up" };

    infoln!("{}", dim(&format!("Binds {}:", action)));
    infoln!("{}", bold(&as_str));
    infoln!()
}

fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

fn dim(s: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", s)
}
