use super::*;
use crate::{
    config::{
        config::{Action, Binding, Gestures, Point},
        window_provider::{DisplayProtocol, MockWindowProvider},
    },
    device::device::{
        EventType, MockDevice, MockVirtualDevice, EVENT_TYPE_KEY, EVENT_TYPE_RELATIVE,
    },
    handler::executor::MockExecutor,
    recognizer::angle_recognizer::AngleRecognizer,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct Dependencies {
    config: Config,
    mouse: MockDevice,
    recognizer: AngleRecognizer,
    window_provider: MockWindowProvider,
    virtual_mouse: MockVirtualDevice,
    virtual_keyboard: MockVirtualDevice,
    executor: MockExecutor,
    mouse_events: Arc<Mutex<Vec<(u16, u16, i32)>>>,
    keyboard_events: Arc<Mutex<Vec<(u16, u16, i32)>>>,
    executor_commands: Arc<Mutex<Vec<String>>>,
}

fn setup(motions: &Vec<(i32, i32)>, app_id: Option<String>) -> Dependencies {
    let points = motions
        .iter()
        .map(|(x, y)| Point { x: *x, y: *y })
        .collect();

    let actions = vec![
        Action {
            app_id: Some("app_id".to_string()),
            modifiers: None,
            keys: Some(vec!["KEY_LEFTCTRL".to_string(), "KEY_C".to_string()]),
            command: Some("some command".to_string()),
        },
        Action {
            app_id: None,
            modifiers: None,
            keys: Some(vec!["KEY_A".to_string(), "KEY_B".to_string()]),
            command: Some("some other command".to_string()),
        },
    ];

    let binds: HashMap<String, Vec<String>> = HashMap::from([(
        "BTN_EXTRA".to_string(),
        vec![
            "KEY_LEFTMETA".to_string(),
            "KEY_C".to_string(),
            "BTN_LEFT".to_string(),
        ],
    )]);

    let config = Config {
        gestures: Gestures {
            button: "BTN_SIDE".to_string(),
            bindings: vec![Binding {
                name: "UP".to_string(),
                actions: actions,
                points: points,
            }],
        },
        binds: binds,
    };

    let mouse_events = Arc::new(Mutex::new(Vec::<(u16, u16, i32)>::new()));
    let mouse_events_clone = Arc::clone(&mouse_events);

    let keyboard_events = Arc::new(Mutex::new(Vec::<(u16, u16, i32)>::new()));
    let keyboard_events_clone = Arc::clone(&keyboard_events);

    let executor_commands = Arc::new(Mutex::new(Vec::<String>::new()));
    let executor_commands_clone = Arc::clone(&executor_commands);

    let mouse = MockDevice::new();
    let recognizer = AngleRecognizer::new();
    let mut window_provider = MockWindowProvider::new();
    let mut virtual_mouse = MockVirtualDevice::new();
    let mut virtual_keyboard = MockVirtualDevice::new();
    let mut executor = MockExecutor::new();

    window_provider
        .expect_get_app_id()
        .returning(move || app_id.clone());
    window_provider
        .expect_get_display_protocol()
        .return_const(DisplayProtocol::Wayland);
    window_provider
        .expect_get_wm()
        .return_const(Some("Test".to_string()));

    virtual_mouse
        .expect_emit()
        .returning(move |event_type, code, value| {
            mouse_events_clone
                .lock()
                .unwrap()
                .push((event_type, code, value));

            Ok(())
        });
    virtual_mouse
        .expect_has_capabilities()
        .returning(|_, code| (256..337).contains(&code));

    virtual_keyboard
        .expect_emit()
        .returning(move |event_type, code, value| {
            keyboard_events_clone
                .lock()
                .unwrap()
                .push((event_type, code, value));

            Ok(())
        });

    virtual_keyboard
        .expect_has_capabilities()
        .returning(|_, code| (1..248).contains(&code));

    executor
        .expect_execute_in_background()
        .returning(move |cmd| {
            executor_commands_clone
                .lock()
                .unwrap()
                .push(cmd.to_string());
        });

    Dependencies {
        config,
        mouse,
        recognizer,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        executor_commands,
    }
}

#[test]
fn test_binds() {
    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        recognizer,
        ..
    } = setup(&vec![], None);

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 276,
            value: 1,
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 276,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_KEY, 272, 1), // BTN_LEFT down
            (EVENT_TYPE_KEY, 272, 0), // BTN_LEFT up
        ],
        "the virtual mouse will simulate a key combination"
    );

    assert_eq!(
        *keyboard_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_KEY, 125, 1), // KEY_LEFTMETA down
            (EVENT_TYPE_KEY, 46, 1),  // KEY_C down
            (EVENT_TYPE_KEY, 125, 0), // KEY_LEFTMETA up
            (EVENT_TYPE_KEY, 46, 0)   // KEY_C up
        ],
        "the virtual keyboard will simulate a key combination"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_gesture_for_matching_app_id() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (214, 928),
        (214, 857),
        (214, 785),
        (214, 714),
        (214, 642),
        (214, 571),
        (214, 500),
        (214, 428),
        (214, 357),
    ];

    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        executor_commands,
        recognizer,
    } = setup(&motions, Some("app_id".to_string()));

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 1,
        });

        motions.iter().for_each(|(x, y)| {
            on_event(EventType::Motion {
                event_type: EVENT_TYPE_RELATIVE,
                code: 1,
                value: 1,
                x: *x,
                y: *y,
            });
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1)
        ],
        "non-consumed events are forwarded to the virtual mouse"
    );

    assert_eq!(
        *keyboard_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_KEY, 29, 1), // KEY_LEFTCTRL down
            (EVENT_TYPE_KEY, 46, 1), // KEY_C down
            (EVENT_TYPE_KEY, 29, 0), // KEY_LEFTCTRL up
            (EVENT_TYPE_KEY, 46, 0)  // KEY_C up
        ],
        "a key combination is forwarded to the virtual keyboard"
    );

    assert_eq!(
        *executor_commands.lock().unwrap(),
        vec!["some command"],
        "the executor executes a command"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_non_relevant_button() {
    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        recognizer,
        ..
    } = setup(&vec![], None);

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 273,
            value: 1,
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 273,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![(EVENT_TYPE_KEY, 273, 1), (EVENT_TYPE_KEY, 273, 0)],
        "the button press and release is forwarded to the virtual mouse"
    );

    assert!(
        keyboard_events.lock().unwrap().is_empty(),
        "nothing is forwarded to the virtual keyboard"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_gesture_for_non_matchin_app_id() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (214, 928),
        (214, 857),
        (214, 785),
        (214, 714),
        (214, 642),
        (214, 571),
        (214, 500),
        (214, 428),
        (214, 357),
    ];

    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        executor_commands,
        recognizer,
    } = setup(&motions, None);

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 1,
        });

        motions.iter().for_each(|(x, y)| {
            on_event(EventType::Motion {
                event_type: EVENT_TYPE_RELATIVE,
                code: 1,
                value: 1,
                x: *x,
                y: *y,
            });
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1)
        ],
        "non-consumed events are forwarded to the virtual mouse"
    );

    assert_eq!(
        *keyboard_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_KEY, 30, 1), // KEY_A down
            (EVENT_TYPE_KEY, 48, 1), // KEY_B down
            (EVENT_TYPE_KEY, 30, 0), // KEY_A up
            (EVENT_TYPE_KEY, 48, 0), // KEY_B up
        ],
        "a key combination is forwarded to the virtual keyboard"
    );

    assert_eq!(
        *executor_commands.lock().unwrap(),
        vec!["some other command"],
        "the executor executes a command"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_no_matching_gesture() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (214, 928),
        (214, 857),
        (214, 785),
        (214, 714),
        (214, 642),
        (214, 571),
        (214, 500),
        (214, 428),
        (214, 357),
    ];

    let non_valid_motions: Vec<(i32, i32)> = vec![
        (1, 2),
        (3, 4),
        (5, 6),
        (7, 8),
        (9, 10),
        (11, 12),
        (13, 14),
        (15, 16),
        (16, 18),
        (19, 20),
        (20, 21),
    ];

    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        executor_commands,
        recognizer,
    } = setup(&motions, None);

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 1,
        });

        non_valid_motions.iter().for_each(|(x, y)| {
            on_event(EventType::Motion {
                event_type: EVENT_TYPE_RELATIVE,
                code: 1,
                value: 1,
                x: *x,
                y: *y,
            });
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1)
        ],
        "non-consumed events are forwarded to the virtual mouse"
    );

    assert!(
        keyboard_events.lock().unwrap().is_empty(),
        "nothing is forwarded to the virtual keyboard"
    );

    assert!(
        executor_commands.lock().unwrap().is_empty(),
        "the executor executes nothing"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_below_threshold() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (214, 928),
        (214, 857),
        (214, 785),
        (214, 714),
        (214, 642),
        (214, 571),
        (214, 500),
        (214, 428),
        (214, 357),
    ];

    let Dependencies {
        config,
        mut mouse,
        window_provider,
        virtual_mouse,
        virtual_keyboard,
        executor,
        mouse_events,
        keyboard_events,
        executor_commands,
        recognizer,
    } = setup(&motions, None);

    mouse.expect_grab().returning(move |on_event| {
        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 1,
        });

        motions[0..6].iter().for_each(|(x, y)| {
            on_event(EventType::Motion {
                event_type: EVENT_TYPE_RELATIVE,
                code: 1,
                value: 1,
                x: *x,
                y: *y,
            });
        });

        on_event(EventType::Key {
            event_type: EVENT_TYPE_KEY,
            code: 275,
            value: 0,
        });
        Ok(())
    });

    let result = start(
        mouse,
        virtual_mouse,
        virtual_keyboard,
        recognizer,
        executor,
        config,
        window_provider,
    );

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_RELATIVE, 1, 1),
            (EVENT_TYPE_KEY, 275, 1),
            (EVENT_TYPE_KEY, 275, 0)
        ],
        "non-consumed events are forwarded to the virtual mouse"
    );

    assert!(
        keyboard_events.lock().unwrap().is_empty(),
        "nothing is forwarded to the virtual keyboard"
    );

    assert!(
        executor_commands.lock().unwrap().is_empty(),
        "the executor will execute nothing"
    );

    assert!(result.is_ok(), "everything is ok");
}
