//! What the app remembers per camera between connects: the learned access
//! point warm-up and when the camera last connected (a recent connect finds
//! the access point still up).

use std::collections::BTreeMap;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nsh_core::prelude::data_dir;
use serde::{Deserialize, Serialize};

use crate::domain::connect_policy::{WARM_RECONNECT_WINDOW, next_warmup};
use crate::domain::{CameraModel, ConnectOptions, ConnectReport};

const FILE: &str = "connect-warmup.json";

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Entry {
    learned_ms: Option<u64>,
    last_connected_ms: Option<u64>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn load() -> BTreeMap<String, Entry> {
    data_dir()
        .and_then(|dir| fs::read(dir.join(FILE)).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(entries: &BTreeMap<String, Entry>) {
    let Some(dir) = data_dir() else { return };
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(&dir)?;
        let temporary = dir.join(format!("{FILE}.tmp"));
        fs::write(&temporary, serde_json::to_vec(entries).unwrap_or_default())?;
        fs::rename(temporary, dir.join(FILE))
    };
    if let Err(error) = write() {
        log::warn!("Could not save connect warm-up: {error}");
    }
}

/// The options for connecting to the camera with this Bluetooth id.
pub fn options_for(camera: &str) -> ConnectOptions {
    let entry = load().get(camera).copied().unwrap_or_default();
    ConnectOptions {
        learned_warmup: entry.learned_ms.map(Duration::from_millis),
        cold_start: entry.last_connected_ms.is_none_or(|last| {
            now_ms().saturating_sub(last) >= WARM_RECONNECT_WINDOW.as_millis() as u64
        }),
        advertised_powered_on: false,
        unseen: false,
    }
}

/// Record a successful connect. Only a cold start teaches the warm-up.
pub fn learn(camera: &str, model: CameraModel, options: ConnectOptions, report: &ConnectReport) {
    let mut entries = load();
    let entry = entries.entry(camera.to_owned()).or_default();
    if options.cold_start {
        let next = next_warmup(
            options.learned_warmup,
            model,
            report.first_join,
            report.first_join_elapsed,
            report.first_join_verified,
        );
        entry.learned_ms = Some(next.as_millis() as u64);
    }
    entry.last_connected_ms = Some(now_ms());
    save(&entries);
}
