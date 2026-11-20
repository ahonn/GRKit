//! Identity discovery writes the camera card only to create its marker once.
//! A missing hardware or card identity disables mutations instead of
//! guessing from a path.
use anyhow::{bail, Context, Result};
use grkit_core::packaging::sha256;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub fn boot_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(format!("boot-{}", sha256(&bytes)))
}

/// The ID of a software request that the service writes to the card: a
/// random UUID.
pub fn request_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

/// The card ID that the service publishes: the hash of the random marker of
/// `card_marker`. A format of the card removes the marker, so the card then
/// gets a new identity. A copy of the whole card keeps the marker: the two
/// cards then have one identity.
pub fn card_id(root: &Path) -> Result<String> {
    let marker = card_marker(root)?;
    Ok(format!(
        "card-{}",
        sha256(format!("grkit-card-1:{marker}").as_bytes())
    ))
}

/// `card_id`, read from the card once in the life of the service: removing
/// the card powers the camera off, so the card of a root cannot change while
/// the service runs.
pub fn session_card_id(root: &Path) -> Result<String> {
    static KNOWN: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);
    let mut known = KNOWN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, id)) = known.as_ref().filter(|(path, _)| path == root) {
        return Ok(id.clone());
    }
    let id = card_id(root)?;
    *known = Some((root.to_path_buf(), id.clone()));
    Ok(id)
}

/// The marker file that is the card identity.
pub const MARKER: &str = "GRKIT/CARD.ID";

/// Read the card marker, or create it once with 128 random bits. This is the
/// only write of identity discovery: one small file in the life of a card.
pub fn card_marker(root: &Path) -> Result<String> {
    let path = root.join(MARKER);
    let read = |path: &Path| -> Result<String> {
        let mut text = String::new();
        fs::File::open(path)?.take(64).read_to_string(&mut text)?;
        let marker = text.trim().to_owned();
        if marker.len() != 32
            || !marker
                .bytes()
                .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
        {
            bail!("The card marker is invalid");
        }
        Ok(marker)
    };
    match read(&path) {
        Ok(marker) => return Ok(marker),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => {}
        Err(error) => return Err(error),
    }
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let marker: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .context("Create the card marker")?;
    file.write_all(marker.as_bytes())?;
    file.sync_all()?;
    drop(file);
    // Read it back: a card that drops the write must not get a new
    // identity at each start.
    let stored = read(&path)?;
    if stored != marker {
        bail!("The card marker did not persist");
    }
    Ok(stored)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_identity_is_the_marker_of_the_card() {
        let root = tempfile::tempdir().unwrap();
        assert!(card_id(root.path()).is_err(), "no GRKIT folder");
        fs::create_dir(root.path().join("GRKIT")).unwrap();
        let id = card_id(root.path()).unwrap();
        assert!(id.starts_with("card-"));
        assert_eq!(card_id(root.path()).unwrap(), id, "created once");
        // A format removes the marker: the next start makes a new identity.
        fs::remove_file(root.path().join(MARKER)).unwrap();
        let formatted = card_id(root.path()).unwrap();
        assert_ne!(formatted, id);
        // The service keeps the identity of a root for its lifetime.
        assert_eq!(session_card_id(root.path()).unwrap(), formatted);
        fs::write(root.path().join(MARKER), "not hex").unwrap();
        assert_eq!(session_card_id(root.path()).unwrap(), formatted);
        assert!(card_id(root.path()).is_err());
    }
}
