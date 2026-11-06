use super::*;

#[test]
fn test_normalize_points() {
    let points = vec![
        Point { x: 0, y: 0 },
        Point { x: 10, y: 0 },
        Point { x: 5, y: 4 },
        Point { x: 5, y: 2 },
    ];

    let norm = normalize(&points, false);
    assert_eq!(norm.get(0).unwrap(), &Point { x: 0, y: 0 });
    assert_eq!(norm.get(1).unwrap(), &Point { x: 1000, y: 0 });
    assert_eq!(norm.get(2).unwrap(), &Point { x: 500, y: 400 });
    assert_eq!(norm.get(3).unwrap(), &Point { x: 500, y: 200 });
}

#[test]
fn test_parse_config() {
    let config_str = r#"
        # some multiline
        # comments

        key = executable cmd --some-flag value | some-other-executable -flag '.with-quotes' --testing=second-quote # some inline comment
    "#;

    let parsed = parse_config_str(config_str);
    let expected = [
        ("key".to_string(), "executable cmd --some-flag value | some-other-executable -flag '.with-quotes' --testing=second-quote".to_string()),
    ].into_iter().collect::<HashMap<_, _>>();

    assert_eq!(parsed, expected);
}

#[test]
fn test_default_config_parses() {
    let data = include_str!("../../resources/config.json");

    let config = serde_json::from_str::<Config>(data).unwrap();

    assert_eq!(config.gestures.button, "BTN_SIDE");
    assert_eq!(config.gestures.bindings.len(), 4);
    assert!(config.gestures.bindings.iter().all(|b| b.points.len() > 100));
}
