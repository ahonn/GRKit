//! The ZIP of a package: its entries within the package limits, its
//! `SHA256SUMS`, and reproducible writing.

use super::*;

pub(crate) fn checksum_list(contents: &Files) -> Vec<u8> {
    contents
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, blob)| format!("{}  {name}\n", sha256(blob)))
        .collect::<String>()
        .into_bytes()
}

/// The time of a ZIP entry with `blob`: a time in 2026 or 2027 that follows from
/// the bytes, so the ZIP stays deterministic and a file whose bytes change
/// gets another time. A copy of the ZIP keeps the times, and the camera takes a
/// file with the size and the time that it recorded as unchanged
/// (`runtime/install_library.h`, `runtime/fastboot/backend.c`). Months have 28
/// days, so each date is valid, and seconds are even, as FAT keeps them.
fn entry_time(blob: &[u8]) -> DateTime {
    let digest = Sha256::digest(blob);
    let hash = u64::from_le_bytes(digest[..8].try_into().expect("8 bytes"));
    let day = (hash % (2 * 12 * 28)) as u16;
    let seconds = ((hash / (2 * 12 * 28)) % (24 * 60 * 30)) as u32 * 2;
    DateTime::from_date_and_time(
        2026 + day / (12 * 28),
        (day % (12 * 28) / 28 + 1) as u8,
        (day % 28 + 1) as u8,
        (seconds / 3600) as u8,
        (seconds / 60 % 60) as u8,
        (seconds % 60) as u8,
    )
    .expect("a valid time")
}

/// A deterministic ZIP of `contents`, with stored entries and the time of
/// each entry from its bytes (`entry_time`).
pub(crate) fn write_zip(contents: Files) -> Result<Vec<u8>> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in contents {
        let mode = if name.ends_with(".BIN") || name.ends_with(".SH") {
            0o755
        } else {
            0o644
        };
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(entry_time(&blob))
            .unix_permissions(mode);
        archive.start_file(name, options)?;
        archive.write_all(&blob)?;
    }
    Ok(archive.finish()?.into_inner())
}

/// The entries of a ZIP within the package limits: safe, unique names, no links, and bounded
/// sizes.
pub(crate) fn read_entries(bytes: &[u8]) -> Result<Files> {
    let declared_entries = declared_entry_count(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Invalid ZIP archive")?;
    // zip indexes names in a map. Check the raw count so duplicate names cannot disappear.
    ensure!(
        archive.len() == declared_entries,
        "Duplicate or inconsistent archive entries"
    );
    let mut contents = Files::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let raw_name = entry.name().context("Invalid archive entry name")?;
        let name = safe_path(&raw_name)?.to_owned();
        ensure!(
            !contents.contains_key(&name)
                && !entry.is_dir()
                && !entry.encrypted()
                && entry.unix_mode().unwrap_or(0) & 0o170000 != 0o120000,
            "Invalid archive entry"
        );
        ensure!(
            entry.size() <= MAX_FILE_SIZE as u64,
            "Archive entry exceeds size limit"
        );
        total = total
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        ensure!(
            total <= MAX_TOTAL_SIZE as u64,
            "Archive exceeds total size limit"
        );
        // Bound actual decompression as well as the untrusted central-directory size.
        let expected_size = entry.size();
        let mut blob = Vec::with_capacity(expected_size as usize);
        (&mut entry)
            .take(MAX_FILE_SIZE as u64 + 1)
            .read_to_end(&mut blob)?;
        ensure!(
            blob.len() as u64 == expected_size,
            "Archive entry size mismatch"
        );
        contents.insert(name, blob);
    }
    Ok(contents)
}

/// The entry `name` of a ZIP, at most `limit` bytes, without reading the other entries.
/// `None` when the ZIP has no such entry.
pub(crate) fn read_entry(bytes: &[u8], name: &str, limit: usize) -> Result<Option<Vec<u8>>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Invalid ZIP archive")?;
    let mut entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => return Err(error).context("Invalid ZIP archive"),
    };
    ensure!(
        entry.size() <= limit as u64 && !entry.encrypted(),
        "Invalid archive entry {name}"
    );
    let mut blob = Vec::new();
    (&mut entry).take(limit as u64 + 1).read_to_end(&mut blob)?;
    ensure!(
        blob.len() as u64 == entry.size(),
        "Archive entry size mismatch"
    );
    Ok(Some(blob))
}

fn declared_entry_count(bytes: &[u8]) -> Result<usize> {
    ensure!(bytes.len() >= 22, "Truncated ZIP archive");
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let end = (start..=bytes.len() - 22)
        .rev()
        .find(|&position| {
            bytes[position..position + 4] == *b"PK\x05\x06"
                && position
                    + 22
                    + u16::from_le_bytes(bytes[position + 20..position + 22].try_into().unwrap())
                        as usize
                    == bytes.len()
        })
        .context("Invalid ZIP end record")?;
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap());
    let count = if count == u16::MAX {
        ensure!(
            end >= 20 && bytes[end - 20..end - 16] == *b"PK\x06\x07",
            "Missing ZIP64 locator"
        );
        let offset = u64::from_le_bytes(bytes[end - 12..end - 4].try_into().unwrap());
        let position = usize::try_from(offset).context("ZIP64 offset exceeds platform limit")?;
        ensure!(
            position <= bytes.len().saturating_sub(56)
                && bytes[position..position + 4] == *b"PK\x06\x06",
            "Invalid ZIP64 end record"
        );
        u64::from_le_bytes(bytes[position + 32..position + 40].try_into().unwrap())
    } else {
        u64::from(count)
    };
    ensure!(count <= MAX_ENTRIES as u64, "Archive exceeds entry limit");
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copied file keeps its ZIP time, and the camera takes a file with the
    /// size and the time that it recorded as unchanged, so files with other
    /// bytes get other times, and the same bytes the same time.
    #[test]
    fn zip_entries_with_other_bytes_get_other_times() {
        let mut times = std::collections::BTreeSet::new();
        for i in 0..400u32 {
            let mut blob = vec![0u8; 8704];
            blob[..4].copy_from_slice(&i.to_le_bytes());
            let t = entry_time(&blob);
            assert!((2026..=2027).contains(&t.year()));
            assert!((1..=12).contains(&t.month()) && (1..=28).contains(&t.day()));
            assert_eq!(t.second() % 2, 0);
            times.insert((
                t.year(),
                t.month(),
                t.day(),
                t.hour(),
                t.minute(),
                t.second(),
            ));
        }
        assert_eq!(times.len(), 400);
        let blob = vec![7u8; 100];
        let (a, b) = (entry_time(&blob), entry_time(&blob));
        assert_eq!(
            (
                a.year(),
                a.month(),
                a.day(),
                a.hour(),
                a.minute(),
                a.second()
            ),
            (
                b.year(),
                b.month(),
                b.day(),
                b.hour(),
                b.minute(),
                b.second()
            )
        );
    }
}
