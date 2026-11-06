use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{Result, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub gestures: Gestures,
    pub binds: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Gestures {
    pub button: String,
    pub bindings: Vec<Binding>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub actions: Vec<Action>,
    pub points: Vec<Point>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Action {
    pub app_id: Option<String>,
    pub modifiers: Option<Vec<String>>,
    pub keys: Option<Vec<String>>,
    pub command: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn normalize(points: &[Point], use_avg: bool) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }

    let (min_x, max_x) = points
        .iter()
        .map(|p| p.x)
        .fold((i32::MAX, i32::MIN), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    let (min_y, max_y) = points
        .iter()
        .map(|p| p.y)
        .fold((i32::MAX, i32::MIN), |(min, max), y| {
            (min.min(y), max.max(y))
        });

    let width = max_x - min_x;
    let height = max_y - min_y;
    let size = width.max(height);

    if width == 0 || height == 0 {
        return Vec::new();
    }

    let (offset_x, offset_y) = if use_avg {
        let sum_x: i32 = points.iter().map(|p| p.x).sum();
        let sum_y: i32 = points.iter().map(|p| p.y).sum();
        let len = points.len() as i32;
        (sum_x / len, sum_y / len)
    } else {
        (min_x, min_y)
    };

    points
        .iter()
        .map(|p| Point {
            x: 1000 * (p.x - offset_x) / size,
            y: 1000 * (p.y - offset_y) / size,
        })
        .collect()
}

pub fn load_window_provider() -> Result<Option<(String, String)>> {
    let map: HashMap<String, String> = load_wm_conf_file()?;
    let Ok(current_desktop) = env::var("XDG_CURRENT_DESKTOP") else {
        return Ok(None);
    };
    let result = map
        .get(&current_desktop)
        .cloned()
        .map(|provider| (current_desktop, provider));

    Ok(result)
}

pub fn load_config() -> Result<Config> {
    let data = load_file_content("config.json", include_bytes!("../../resources/config.json"))?;
    let config = serde_json::from_str::<Config>(&data)?;

    Ok(config)
}

fn load_wm_conf_file() -> Result<HashMap<String, String>> {
    let data = load_file_content("wm.conf", include_bytes!("../../resources/wm.conf"))?;
    let map = parse_config_str(&data);

    Ok(map)
}

fn load_file_content(name: &str, default_content: &'static [u8]) -> Result<String> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("mouse-gestures");

    let config_path = xdg_dirs
        .place_config_file(name)
        .expect("cannot create configuration directory");

    if !Path::new(&config_path).exists() {
        File::create(&config_path)?.write_all(default_content)?;
    }

    fs::read_to_string(config_path)
}

fn parse_config_str(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::<String, String>::new();

    for line in input.lines() {
        let mut line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(pos) = line.find('#') {
            line = &line[..pos].trim_end();
        }

        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            map.insert(key, value);
        }
    }

    map
}

#[cfg(test)]
#[path = "./__config_test.rs"]
mod test;
