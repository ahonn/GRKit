//! Verify the release evidence of the card: it names the official Looks.
use anyhow::{ensure, Context, Result};
use grkit_core::release::{self, Statement};
use std::{fs, path::Path};

pub fn installed_release(root: &Path, keys: &[[u8; 32]]) -> Result<Statement> {
    let read = |path: &str, limit: usize| -> Result<Vec<u8>> {
        let path = root.join(path);
        ensure!(
            fs::symlink_metadata(&path)?.file_type().is_file(),
            "Release evidence must be a regular file"
        );
        Ok(grkit_core::bounded::read_to_limit(
            fs::File::open(&path)?,
            limit,
        )?)
    };
    let statement = read(release::CARD_STATEMENT, release::MAX_STATEMENT_BYTES)
        .context("Read the installed release statement")?;
    let signature = read(release::CARD_SIGNATURE, release::MAX_SIGNATURE_BYTES)
        .context("Read the installed release signature")?;
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
