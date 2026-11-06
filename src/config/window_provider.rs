use crate::config::config::load_window_provider;
use std::env;
use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};
use std::process::{Command, Stdio};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

#[derive(Debug, Clone, Copy)]
pub enum DisplayProtocol {
    Wayland,
    X11,
}

impl Display for DisplayProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayProtocol::Wayland => f.write_str("Wayland"),
            DisplayProtocol::X11 => f.write_str("X11"),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
pub trait WindowProvider {
    fn get_app_id(&self) -> Option<String>;
    fn get_wm(&self) -> Option<String>;
    fn is_valid(&self) -> bool;
    fn get_display_protocol(&self) -> DisplayProtocol;
}

pub struct WaylandWindowProvider {
    wm: Option<String>,
    cmd: Option<String>,
}

impl WaylandWindowProvider {
    pub fn new() -> Result<Self> {
        let (wm, cmd) = match load_window_provider()? {
            Some((wm, cmd)) => (Some(wm), Some(cmd)),
            None => (None, None),
        };

        Ok(Self { wm, cmd })
    }

    fn execute(&self, command: &str) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err_msg = format!("Command failed: {}", stderr.trim_end());

            return Err(Error::new(ErrorKind::Other, err_msg));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(stdout.trim_end().to_string())
    }
}

impl WindowProvider for WaylandWindowProvider {
    fn get_app_id(&self) -> Option<String> {
        self.cmd
            .as_ref()
            .and_then(|cmd| self.execute(cmd).ok())
            .filter(|app_id| !app_id.is_empty())
    }

    fn get_wm(&self) -> Option<String> {
        return self.wm.clone();
    }

    fn is_valid(&self) -> bool {
        self.wm.is_some() && self.cmd.is_some()
    }

    fn get_display_protocol(&self) -> DisplayProtocol {
        return DisplayProtocol::Wayland;
    }
}

pub struct X11WindowProvider {
    conn: RustConnection,
    root: u32,
}

impl X11WindowProvider {
    pub fn new() -> Result<Self> {
        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        let root = conn.setup().roots[screen_num].root;

        Ok(Self { conn, root })
    }

    fn get_window_manager(&self) -> Option<String> {
        // Intern atoms
        let net_supporting_wm_check = self
            .conn
            .intern_atom(false, b"_NET_SUPPORTING_WM_CHECK")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let net_wm_name = self
            .conn
            .intern_atom(false, b"_NET_WM_NAME")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let utf8_string = self
            .conn
            .intern_atom(false, b"UTF8_STRING")
            .ok()?
            .reply()
            .ok()?
            .atom;

        // Get the check window
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                net_supporting_wm_check,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .ok()?
            .reply()
            .ok()?;
        let wm_window = reply.value32().and_then(|mut v| v.next())?;

        // Get the window manager name
        let reply = self
            .conn
            .get_property(false, wm_window, net_wm_name, utf8_string, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        let name = String::from_utf8(reply.value).ok()?;

        Some(name)
    }

    fn get_active_window(&self) -> Option<Window> {
        let net_active_window = self
            .conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()?
            .reply()
            .ok()?
            .atom;

        let reply = self
            .conn
            .get_property(false, self.root, net_active_window, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;

        if reply.value_len == 0 {
            return None;
        }

        let window_id = u32::from_ne_bytes(reply.value[0..4].try_into().ok()?);
        Some(window_id)
    }

    fn get_wm_class(&self, window: Window) -> Option<(String, String)> {
        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
            .ok()?
            .reply()
            .ok()?;

        if reply.value_len == 0 {
            return None;
        }

        let data = String::from_utf8_lossy(&reply.value);
        let mut parts = data.split('\0').filter(|s| !s.is_empty());
        let instance = parts.next()?.to_string();
        let class = parts.next()?.to_string();
        Some((instance, class))
    }
}

impl WindowProvider for X11WindowProvider {
    fn get_app_id(&self) -> Option<String> {
        let Some(id) = self.get_active_window() else {
            return None;
        };
        let Some((_, class)) = self.get_wm_class(id) else {
            return None;
        };

        Some(class)
    }

    fn get_wm(&self) -> Option<String> {
        return self.get_window_manager();
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn get_display_protocol(&self) -> DisplayProtocol {
        return DisplayProtocol::X11;
    }
}

pub struct EnvWindowProvider {
    provider: Box<dyn WindowProvider>,
}

impl EnvWindowProvider {
    pub fn new() -> std::io::Result<Self> {
        let provider: Option<Box<dyn WindowProvider>> = if env::var("WAYLAND_DISPLAY").is_ok() {
            Some(Box::new(WaylandWindowProvider::new()?))
        } else if env::var("DISPLAY").is_ok() {
            Some(Box::new(X11WindowProvider::new()?))
        } else {
            None
        };

        if let Some(provider) = provider {
            Ok(Self { provider })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No suitable provider found",
            ))
        }
    }
}

impl WindowProvider for EnvWindowProvider {
    fn get_app_id(&self) -> Option<String> {
        self.provider.get_app_id()
    }

    fn get_wm(&self) -> Option<String> {
        self.provider.get_wm()
    }

    fn is_valid(&self) -> bool {
        self.provider.is_valid()
    }

    fn get_display_protocol(&self) -> DisplayProtocol {
        return self.provider.get_display_protocol();
    }
}
