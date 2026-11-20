//! Verify the release evidence of the card: it names the official Looks.
use anyhow::{ensure, Context, Result};
use grkit_core::release::{self, Statement};
use std::{fs, io::Read, path::Path};

pub fn installed_release(root: &Path, keys: &[[u8; 32]]) -> Result<Statement> {
    let read = |path: &str, limit: u64| -> Result<Vec<u8>> {
        let path = root.join(path);
        ensure!(
            fs::symlink_metadata(&path)?.file_type().is_file(),
            "Release evidence must be a regular file"
        );
        let mut bytes = Vec::new();
        fs::File::open(&path)?
            .take(limit + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= limit,
            "Release evidence exceeds its size limit"
        );
        Ok(bytes)
    };
    let statement = read(release::CARD_STATEMENT, 128 * 1024)
        .context("Read the installed release statement")?;
    let signature =
        read(release::CARD_SIGNATURE, 1024).context("Read the installed release signature")?;
    release::verify(&statement, &signature, keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use grkit_core::{camera, packaging, testing};
    #[test]
    fn card_reader_package_retains_verifiable_release_evidence() {
        let zip = testing::signed_package(&camera::GR3, "1.7.0", &["test"]);
        let root = tempfile::tempdir().unwrap();
        for (path, bytes) in packaging::card_files(&zip).unwrap() {
            let path = root.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let release = installed_release(root.path(), &[testing::release_key()]).unwrap();
        assert!(!release.runtime.is_empty());
        assert!(installed_release(root.path(), &release::OFFICIAL_KEYS).is_err());
        fs::write(root.path().join(release::CARD_SIGNATURE), b"invalid").unwrap();
        assert!(installed_release(root.path(), &[testing::release_key()]).is_err());
    }
}
