use crate::config::config::Action;
use crate::config::window_provider::WindowProvider;
use crate::device::device;
use crate::device::device::{VirtualDevice, EVENT_TYPE_KEY};
use crate::handler::executor::Executor;
use crate::handler::handler::Handler;
use crate::info;
use crate::{
    config::config::{Config, Point},
    recognizer::recognizer::Recognizer,
};
use linked_hash_set::LinkedHashSet;
use std::thread;
use std::time::Duration;

pub struct GestureHandler<R, E>
where
    R: Recognizer,
    E: Executor,
{
    recognizer: R,
    executor: E,
    record: bool,
    trail: Vec<Point>,
    keys: LinkedHashSet<u16>,
    threshold: usize,
}

impl<R, E> GestureHandler<R, E>
where
    R: Recognizer,
    E: Executor,
{
    pub fn new(recognizer: R, executor: E, threshold: usize) -> Self {
        return Self {
            record: false,
            trail: Vec::new(),
            keys: LinkedHashSet::new(),
            recognizer: recognizer,
            executor: executor,
            threshold: threshold,
        };
    }

    fn reset(&mut self) {
        self.trail.clear();
        self.keys.clear();
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

    fn event_values(&self, keys: &Vec<String>, value: i32) -> Vec<(u16, u16, i32)> {
        keys.iter()
            .filter_map(|key| device::values_from_str(key, value))
            .collect::<Vec<_>>()
    }

    fn run_action(
        &mut self,
        virtual_mouse: &mut impl VirtualDevice,
        virtual_keyboard: &mut impl VirtualDevice,
        action: &Action,
    ) {
        if let Some(keys) = &action.keys {
            info::keys(keys);

            for (event_type, code, value) in self.event_values(keys, 1) {
                self.send_event(virtual_mouse, virtual_keyboard, event_type, code, value);
            }

            thread::sleep(Duration::from_millis(8));

            for (event_type, code, value) in self.event_values(keys, 0) {
                self.send_event(virtual_mouse, virtual_keyboard, event_type, code, value);
            }
        }

        if let Some(command) = &action.command {
            info::command(command);

            let _ = self.executor.execute_in_background(command);
        }
    }

    fn evaluate(
        &mut self,
        config: &Config,
        window_provider: &impl WindowProvider,
        virtual_mouse: &mut impl VirtualDevice,
        virtual_keyboard: &mut impl VirtualDevice,
    ) -> bool {
        let candidate = match self
            .recognizer
            .find_candidate(&self.trail, &config.gestures.bindings)
        {
            Some(c) => c,
            None => return false,
        };

        let app_id = window_provider.get_app_id();

        // Collect pressed modifier keys as strings (convert Option<Vec<_>> → Vec<_>>)
        let modifiers = self
            .keys
            .iter()
            .map(device::str_from_code)
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default();

        // Helper closure to check if modifiers match exactly
        let modifiers_match = |action: &Action| match &action.modifiers {
            Some(required) => required
                .iter()
                .map(String::as_str)
                .eq(modifiers.iter().copied()),
            None => modifiers.is_empty(),
        };

        // Build an iterator of all actions that are eligible for the current app
        let actions = candidate.binding.actions.iter().filter(|a| {
            a.app_id
                .as_ref()
                .map_or(true, |id| Some(id) == app_id.as_ref())
        });

        // Find the *best* action lazily
        let best_action = actions
            // Try exact modifier matches first
            .filter(|a| modifiers_match(a))
            .max_by_key(|a| {
                let app_match = (a.app_id.as_ref() == app_id.as_ref()) as u8;
                let has_mods = a.modifiers.is_some() as u8;
                (app_match << 1) | has_mods // app match = 2 pts, modifiers = 1 pt
            })
            // If none matched, try actions without modifiers
            .or_else(|| {
                candidate
                    .binding
                    .actions
                    .iter()
                    .filter(|a| {
                        a.modifiers.is_none()
                            && a.app_id
                                .as_ref()
                                .map_or(true, |id| Some(id) == app_id.as_ref())
                    })
                    .max_by_key(|a| (a.app_id.as_ref() == app_id.as_ref()) as u8)
            });

        let Some(action) = best_action else {
            return false;
        };

        if let Some(current_app) = &app_id {
            if action.app_id.is_some() {
                info::app_id(current_app);
            }
        }

        if !modifiers.is_empty() {
            info::modifiers(&modifiers);
        }

        self.run_action(virtual_mouse, virtual_keyboard, action);
        true
    }

    fn handle(
        &mut self,
        config: &Config,
        window_provider: &impl WindowProvider,
        virtual_mouse: &mut impl VirtualDevice,
        virtual_keyboard: &mut impl VirtualDevice,
        code: u16,
        value: i32,
    ) -> bool {
        let Some(expected_code) = device::code_from_str(&config.gestures.button) else {
            return false;
        };

        if expected_code != code {
            return self.record_modifiers(code);
        }

        if expected_code != code {
            return false;
        }

        match value {
            1 => {
                self.record = true;
            }
            0 => {
                self.record = false;

                let over_threshold = self.trail.len() > self.threshold;
                let triggered =
                    self.evaluate(config, window_provider, virtual_mouse, virtual_keyboard);
                self.reset();

                // No action has been run. Simulate a mouse click.
                if !triggered && !over_threshold {
                    let _ = virtual_mouse.emit(EVENT_TYPE_KEY, code, 1);
                    let _ = virtual_mouse.emit(EVENT_TYPE_KEY, code, 0);
                }
            }
            _ => {}
        }
        true
    }

    fn record_modifiers(&mut self, code: u16) -> bool {
        if !self.record {
            return false;
        }
        self.keys.insert(code);

        return true;
    }

    fn record_trail(&mut self, point: Point) {
        if self.record {
            self.trail.push(point);
        }
    }
}

impl<R, E, VM, VK, WP> Handler<R, VM, VK, WP> for GestureHandler<R, E>
where
    R: Recognizer,
    E: Executor,
    VM: VirtualDevice,
    VK: VirtualDevice,
    WP: WindowProvider,
{
    fn on_motion(&mut self, x: i32, y: i32) {
        self.record_trail(Point { x, y })
    }

    fn on_key(
        &mut self,
        config: &Config,
        window_provider: &WP,
        virtual_mouse: &mut VM,
        virtual_keyboard: &mut VK,
        code: u16,
        value: i32,
    ) -> bool {
        self.handle(
            config,
            window_provider,
            virtual_mouse,
            virtual_keyboard,
            code,
            value,
        )
    }

    fn on_wheel(&mut self, _: u16, virtual_code: u16, _: i32) -> bool {
        self.record_modifiers(virtual_code)
    }
}
