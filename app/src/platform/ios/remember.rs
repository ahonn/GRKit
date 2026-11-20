//! Load and save the saved cameras in the app data directory.

use std::fs;
use std::path::PathBuf;

use nsh_core::prelude::data_dir;

use crate::domain::{RememberedCamera, decode_saved, encode_saved};

const FILE: &str = "saved_cameras.json";

fn path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(FILE))
}

pub fn load() -> Vec<RememberedCamera> {
    path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| decode_saved(&text).ok())
        .unwrap_or_default()
}

pub fn save(cameras: &[RememberedCamera]) -> Result<(), String> {
    let dir = data_dir().ok_or_else(|| "no app data directory".to_owned())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = encode_saved(cameras)?;
    fs::write(dir.join(FILE), text).map_err(|e| e.to_string())
}
