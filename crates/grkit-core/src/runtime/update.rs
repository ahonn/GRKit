//! An update of the package over Wi-Fi: the journal that installs it at the
//! next power-on.
//!
//! The service on Linux (crates/grkit-serve) writes the new files of the
//! package to `card::UPDATE_FOLDER`, at their paths in the package folder,
//! and `SCRIPT/STARTUP.TTL` there as `STARTUP.TTL`. Then it writes the
//! journal (`journal`) as `card::JOURNAL_FILE`. The card cannot rename a
//! folder or replace a file, so the journal moves the files one at a time:
//! each file that a new file replaces, and each file that the new package
//! does not have, to `card::BACKUP_FOLDER`, then each new file to its place.
//!
//! `SCRIPT/STARTUP.TTL` runs the journal before the loader
//! (`startup_script`). Each step looks at the files before it moves one, so a
//! journal that stops, for example at a power failure, does the rest at the
//! next power-on. The journal deletes itself only after each file is in its
//! place. While it remains, the startup script does not run the loader: the
//! camera starts without GRKit until the update is complete.

use crate::card::{BACKUP_FOLDER, JOURNAL_FILE, PACKAGE_FOLDER, STARTUP_FILE, UPDATE_FOLDER};
use anyhow::{ensure, Result};
use std::collections::BTreeSet;

/// What the journal does with a file of the package folder, or with
/// `SCRIPT/STARTUP.TTL`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Put the new file in its place. The file that it replaces, if any,
    /// goes to the backup folder.
    Install(String),
    /// Move the file to the backup folder: the new package does not have it.
    Remove(String),
}

impl Change {
    fn path(&self) -> &str {
        match self {
            Change::Install(path) | Change::Remove(path) => path,
        }
    }
}

/// The paths of a card file in the update and the backup folders: a file of
/// the package folder keeps its path in the folder, and the startup script
/// is `STARTUP.TTL` in each.
pub fn update_paths(path: &str) -> Result<(String, String)> {
    let inner = if path == STARTUP_FILE {
        "STARTUP.TTL"
    } else {
        path.strip_prefix(PACKAGE_FOLDER)
            .and_then(|rest| rest.strip_prefix('/'))
            .filter(|rest| !rest.is_empty())
            .ok_or_else(|| anyhow::anyhow!("{path} is not a file of the package"))?
    };
    ensure!(
        format!("{PACKAGE_FOLDER}/{inner}") != JOURNAL_FILE,
        "The journal is not a file of the package"
    );
    Ok((
        format!("{UPDATE_FOLDER}/{inner}"),
        format!("{BACKUP_FOLDER}/{inner}"),
    ))
}

/// The card path `path` as the startup script names it, from the root of
/// drive `C:`.
fn rtos(path: &str) -> String {
    format!("C:\\{}", path.replace('/', "\\"))
}

/// The folders that `path` is in, from the root.
fn folders_of(path: &str) -> impl Iterator<Item = String> + '_ {
    path.match_indices('/')
        .map(move |(at, _)| path[..at].to_owned())
}

/// The journal that makes the changes `changes`, as TTL.
pub fn journal(changes: &[Change]) -> Result<Vec<u8>> {
    let mut seen = BTreeSet::new();
    let mut folders = BTreeSet::new();
    let mut steps = Vec::new();
    for change in changes {
        let live = change.path();
        ensure!(seen.insert(live.to_owned()), "Two changes of {live}");
        let (new, old) = update_paths(live)?;
        folders.extend(folders_of(live).chain(folders_of(&old)));
        steps.push((change, live, new, old));
    }
    let mut lines = Vec::new();
    for folder in folders {
        lines.push(format!("foldersearch '{}'", rtos(&folder)));
        lines.push(format!("if result = 0 foldercreate '{}'", rtos(&folder)));
    }
    for (index, (change, live, new, old)) in steps.iter().enumerate() {
        let (live, new, old) = (rtos(live), rtos(new), rtos(old));
        let next = format!("update_{index}");
        match change {
            // A new file not yet in its place: back up the file that it
            // replaces, unless that is done, then move the new file.
            Change::Install(_) => lines.extend([
                format!("filesearch '{new}'"),
                format!("if result = 0 goto {next}"),
                format!("filesearch '{old}'"),
                format!("if result <> 0 goto {next}_move"),
                format!("filesearch '{live}'"),
                format!("if result = 0 goto {next}_move"),
                format!("filerename '{live}' '{old}'"),
                format!(":{next}_move"),
                format!("filerename '{new}' '{live}'"),
            ]),
            Change::Remove(_) => lines.extend([
                format!("filesearch '{live}'"),
                format!("if result = 0 goto {next}"),
                format!("filesearch '{old}'"),
                format!("if result <> 0 goto {next}"),
                format!("filerename '{live}' '{old}'"),
            ]),
        }
        lines.push(format!(":{next}"));
    }
    // `filerename` gives the same result for success and failure, so the
    // journal looks at each file before it deletes itself.
    for (change, live, new, _) in &steps {
        match change {
            Change::Install(_) => lines.push(format!("filesearch '{}'", rtos(new))),
            Change::Remove(_) => lines.push(format!("filesearch '{}'", rtos(live))),
        }
        lines.push("if result <> 0 goto update_incomplete".into());
    }
    lines.extend([
        format!("filedelete '{}'", rtos(JOURNAL_FILE)),
        ":update_incomplete".into(),
    ]);
    Ok((lines.join("\r\n") + "\r\n").into_bytes())
}

/// `SCRIPT/STARTUP.TTL`: it runs the journal of an update, then the loader
/// at `loader` (a path from `C:`) if the journal is complete.
pub fn startup_script(loader: &str) -> String {
    let journal = rtos(JOURNAL_FILE);
    [
        format!("filesearch '{journal}'"),
        "if result = 0 goto update_done".into(),
        format!("include '{journal}'"),
        format!("filesearch '{journal}'"),
        "if result <> 0 goto done".into(),
        ":update_done".into(),
        format!("filesearch '{loader}'"),
        "if result = 0 goto done".into(),
        format!("include '{loader}'"),
        ":done".into(),
        "end".into(),
    ]
    .join("\r\n")
        + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(changes: &[Change]) -> Vec<String> {
        String::from_utf8(journal(changes).unwrap())
            .unwrap()
            .split("\r\n")
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn each_file_has_its_place_in_the_update_and_backup_folders() {
        assert_eq!(
            update_paths("SCRIPT/GRKIT/MODULES/GLOW.GRM").unwrap(),
            (
                "SCRIPT/GRKITNEW/MODULES/GLOW.GRM".into(),
                "SCRIPT/GRKITOLD/MODULES/GLOW.GRM".into()
            )
        );
        assert_eq!(
            update_paths("SCRIPT/STARTUP.TTL").unwrap(),
            (
                "SCRIPT/GRKITNEW/STARTUP.TTL".into(),
                "SCRIPT/GRKITOLD/STARTUP.TTL".into()
            )
        );
        for path in [
            "SCRIPT/GRKIT/COMMIT.TTL",
            "GRKIT/STATE.TXT",
            "SCRIPT/GRKIT",
            "SCRIPT/GRKITX/A",
        ] {
            assert!(update_paths(path).is_err(), "{path}");
        }
    }

    #[test]
    fn the_journal_backs_up_and_moves_each_file_once() {
        let lines = text(&[
            Change::Install("SCRIPT/GRKIT/CORE1.BIN".into()),
            Change::Remove("SCRIPT/GRKIT/EFFECTS/A.GRF".into()),
        ]);
        let at = |line: &str| lines.iter().position(|l| l == line).unwrap();
        // The folders of the files and their backups first.
        assert!(at("foldersearch 'C:\\SCRIPT\\GRKITOLD\\EFFECTS'") < at(":update_0"));
        assert!(
            lines.contains(&"if result = 0 foldercreate 'C:\\SCRIPT\\GRKIT\\EFFECTS'".to_owned())
        );
        assert_eq!(
            lines[at("filesearch 'C:\\SCRIPT\\GRKITNEW\\CORE1.BIN'")..=at(":update_0")],
            [
                "filesearch 'C:\\SCRIPT\\GRKITNEW\\CORE1.BIN'",
                "if result = 0 goto update_0",
                "filesearch 'C:\\SCRIPT\\GRKITOLD\\CORE1.BIN'",
                "if result <> 0 goto update_0_move",
                "filesearch 'C:\\SCRIPT\\GRKIT\\CORE1.BIN'",
                "if result = 0 goto update_0_move",
                "filerename 'C:\\SCRIPT\\GRKIT\\CORE1.BIN' 'C:\\SCRIPT\\GRKITOLD\\CORE1.BIN'",
                ":update_0_move",
                "filerename 'C:\\SCRIPT\\GRKITNEW\\CORE1.BIN' 'C:\\SCRIPT\\GRKIT\\CORE1.BIN'",
                ":update_0",
            ]
        );
        assert_eq!(
            lines[at(":update_0") + 1..=at(":update_1")],
            [
                "filesearch 'C:\\SCRIPT\\GRKIT\\EFFECTS\\A.GRF'",
                "if result = 0 goto update_1",
                "filesearch 'C:\\SCRIPT\\GRKITOLD\\EFFECTS\\A.GRF'",
                "if result <> 0 goto update_1",
                "filerename 'C:\\SCRIPT\\GRKIT\\EFFECTS\\A.GRF' 'C:\\SCRIPT\\GRKITOLD\\EFFECTS\\A.GRF'",
                ":update_1",
            ]
        );
        // It deletes itself only when no new file waits and no removed file
        // stays.
        assert_eq!(
            lines[at(":update_1") + 1..],
            [
                "filesearch 'C:\\SCRIPT\\GRKITNEW\\CORE1.BIN'",
                "if result <> 0 goto update_incomplete",
                "filesearch 'C:\\SCRIPT\\GRKIT\\EFFECTS\\A.GRF'",
                "if result <> 0 goto update_incomplete",
                "filedelete 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'",
                ":update_incomplete",
            ]
        );
    }

    #[test]
    fn a_file_changes_once() {
        assert!(journal(&[
            Change::Install("SCRIPT/GRKIT/A.BIN".into()),
            Change::Remove("SCRIPT/GRKIT/A.BIN".into()),
        ])
        .is_err());
    }

    #[test]
    fn the_startup_script_runs_a_complete_journal_before_the_loader() {
        let script = startup_script("C:\\SCRIPT\\GRKIT\\LOADER.TTL");
        let lines: Vec<&str> = script.split("\r\n").collect();
        assert_eq!(
            lines,
            [
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'",
                "if result = 0 goto update_done",
                "include 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'",
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'",
                "if result <> 0 goto done",
                ":update_done",
                "filesearch 'C:\\SCRIPT\\GRKIT\\LOADER.TTL'",
                "if result = 0 goto done",
                "include 'C:\\SCRIPT\\GRKIT\\LOADER.TTL'",
                ":done",
                "end",
                "",
            ]
        );
    }
}
