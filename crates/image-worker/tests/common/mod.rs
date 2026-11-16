//! Shared helpers for the integration tests: fixture paths and manifest facts.

#![allow(dead_code)]

use std::path::PathBuf;

use image_worker::ImageFormat;
use serde_json::Value;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> PathBuf {
    fixtures_dir().join(rel)
}

pub struct Entry {
    pub path: String,
    pub format: Option<ImageFormat>,
    pub claimed: ImageFormat,
    pub valid: bool,
    pub committed: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u8>,
}

fn format(s: &str) -> Option<ImageFormat> {
    match s {
        "jpeg" => Some(ImageFormat::Jpeg),
        "png" => Some(ImageFormat::Png),
        "heic" => Some(ImageFormat::Heic),
        "none" => None,
        other => panic!("unknown format {other} in manifest"),
    }
}

pub fn manifest() -> Vec<Entry> {
    let text = std::fs::read_to_string(fixture("manifest.json")).expect("manifest");
    let v: Value = serde_json::from_str(&text).expect("manifest json");
    v["files"]
        .as_array()
        .expect("files")
        .iter()
        .filter(|f| f.get("source").is_none())
        .map(|f| Entry {
            path: f["path"].as_str().unwrap().to_string(),
            format: format(f["format"].as_str().unwrap()),
            claimed: format(f["claimed_format"].as_str().unwrap()).unwrap(),
            valid: f["valid"].as_bool().unwrap(),
            committed: f["committed"].as_bool().unwrap(),
            width: f["width"].as_u64().map(|w| w as u32),
            height: f["height"].as_u64().map(|h| h as u32),
            orientation: f["orientation"].as_u64().map(|o| o as u8),
        })
        .collect()
}

pub fn entry(path: &str) -> Entry {
    manifest()
        .into_iter()
        .find(|e| e.path == path)
        .unwrap_or_else(|| panic!("{path} not in manifest"))
}
