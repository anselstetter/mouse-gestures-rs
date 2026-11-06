use crate::{
    cmd::trail::start,
    config::config::{normalize, Point},
    device::device::{
        EventType, MockDevice, MockVirtualDevice, EVENT_TYPE_KEY, EVENT_TYPE_RELATIVE,
    },
    writer::MockWriter,
};
use std::sync::{Arc, Mutex};

struct Dependencies {
    mouse: MockDevice,
    virtual_mouse: MockVirtualDevice,
    writer: MockWriter,
    mouse_events: Arc<Mutex<Vec<(u16, u16, i32)>>>,
    writer_lines: Arc<Mutex<Vec<String>>>,
}

fn setup() -> Dependencies {
    let mouse_events = Arc::new(Mutex::new(Vec::<(u16, u16, i32)>::new()));
    let mouse_events_clone = Arc::clone(&mouse_events);

    let writer_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let writer_lines_clone = Arc::clone(&writer_lines);

    let mouse = MockDevice::new();
    let mut virtual_mouse = MockVirtualDevice::new();
    let mut writer = MockWriter::new();

    virtual_mouse
        .expect_emit()
        .returning(move |event_type, code, value| {
            mouse_events_clone
                .lock()
                .unwrap()
                .push((event_type, code, value));

            Ok(())
        });

    writer
        .expect_write()
        .returning(move |s| writer_lines_clone.lock().unwrap().push(s.to_string()));

    Dependencies {
        mouse,
        virtual_mouse,
        writer,
        mouse_events,
        writer_lines,
    }
}

#[test]
fn test_trail_with_right_mouse_button() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (215, 928),
        (216, 857),
        (217, 785),
        (218, 714),
        (219, 642),
        (220, 571),
        (221, 500),
        (222, 428),
        (223, 357),
    ];

    let points = motions
        .iter()
        .map(|(x, y)| Point { x: *x, y: *y })
        .collect::<Vec<_>>();

    let Dependencies {
        mut mouse,
        virtual_mouse,
        writer,
        mouse_events,
        writer_lines,
    } = setup();

    mouse
        .expect_grab_nonblock()
        .returning(move |on_event, _, _| {
            on_event(EventType::Key {
                event_type: EVENT_TYPE_KEY,
                code: 272,
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
                code: 272,
                value: 0,
            });
            Ok(())
        });

    let result = start(mouse, virtual_mouse, writer);

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
        "non consumed events are forwarded to the virtual mouse"
    );

    let normalized = normalize(&points, false);
    let json = serde_json::to_string(&normalized).unwrap();

    assert_eq!(
        *writer_lines.lock().unwrap(),
        vec![json],
        "a normalized trail is written to the writer"
    );

    assert!(result.is_ok(), "everything is ok");
}

#[test]
fn test_trail_with_wrong_mouse_button() {
    let motions: Vec<(i32, i32)> = vec![
        (214, 1000),
        (215, 928),
        (216, 857),
        (217, 785),
        (218, 714),
        (219, 642),
        (220, 571),
        (221, 500),
        (222, 428),
        (223, 357),
    ];

    let Dependencies {
        mut mouse,
        virtual_mouse,
        writer,
        mouse_events,
        writer_lines,
    } = setup();

    mouse
        .expect_grab_nonblock()
        .returning(move |on_event, _, _| {
            on_event(EventType::Key {
                event_type: EVENT_TYPE_KEY,
                code: 274,
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
                code: 274,
                value: 0,
            });
            Ok(())
        });

    let result = start(mouse, virtual_mouse, writer);

    assert_eq!(
        *mouse_events.lock().unwrap(),
        vec![
            (EVENT_TYPE_KEY, 274, 1),
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
            (EVENT_TYPE_KEY, 274, 0)
        ],
        "non consumed events are forwarded to the virtual mouse"
    );

    assert_eq!(
        *writer_lines.lock().unwrap(),
        vec!["[]"],
        "an empty array is written to the writer"
    );

    assert!(result.is_ok(), "everything is ok");
}
