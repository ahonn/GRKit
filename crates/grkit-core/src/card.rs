//! The folders and files of GRKit on the SD card. A package names a file by
//! its path on the card, the installers on the camera's Linux, and the loader
//! in the RTOS (`C:\`). The three spellings of each folder come from one
//! literal here.

/// The package folder, with `$separator` between its parts.
macro_rules! package_folder {
    ($separator:literal) => {
        concat!("SCRIPT", $separator, "GRKIT")
    };
}
/// The data folder.
macro_rules! data_folder {
    () => {
        "GRKIT"
    };
}
/// The card on the camera's Linux.
macro_rules! linux_card {
    () => {
        "/media/storage1/"
    };
}

/// The card root on Linux.
pub const LINUX_CARD: &str = "/media/storage1";
/// The startup script of the firmware, which runs the loader.
pub const STARTUP_FILE: &str = "SCRIPT/STARTUP.TTL";
/// An update over Wi-Fi (`runtime::update`): the new files wait in
/// `UPDATE_FOLDER` at their paths in the package folder, and the files that
/// they replace or remove go to `BACKUP_FOLDER`, because the card cannot
/// replace a file. `JOURNAL_FILE` moves them at the next power-on, before the
/// loader runs. The service deletes the old files after a successful update.
pub const UPDATE_FOLDER: &str = concat!(package_folder!("/"), "NEW");
pub const BACKUP_FOLDER: &str = concat!(package_folder!("/"), "OLD");
pub const JOURNAL_FILE: &str = concat!(package_folder!("/"), "/COMMIT.TTL");
/// The record of the software request of the last update, in
/// `BACKUP_FOLDER`. The service keeps it when it deletes the old files.
pub const BACKUP_RECORD: &str = concat!(package_folder!("/"), "OLD/RECORD.JSN");
/// The journal of an update that waits for the confirmation on the camera.
/// The resident loader renames it to `JOURNAL_FILE` after the confirmation.
pub const REQUEST_FILE: &str = concat!(package_folder!("/"), "/COMMIT.REQ");
/// The version of the package of an update, as six digits: two each for the
/// major, minor, and patch numbers, such as `010700` for 1.7.0. The service
/// writes it before `REQUEST_FILE`. It stays after the install, so the next
/// start tells that an update installed, and the resident loader deletes it.
pub const VERSION_FILE: &str = concat!(package_folder!("/"), "/COMMIT.VER");
/// The journal while the service writes it. The service renames it to
/// `REQUEST_FILE` after it wrote all of it.
pub const DRAFT_FILE: &str = concat!(package_folder!("/"), "/COMMIT.TMP");

/// The package folder on the card: the loader, the core, the package list,
/// the package modules in `MODULES`, and the inbox of the package effects in
/// `EFFECTS`, which SCAN copies into the effects library.
pub const PACKAGE_FOLDER: &str = package_folder!("/");
/// The package folder on Linux: the installers, their scripts, the package
/// files, and the files of each boot.
pub const LINUX_DIRECTORY: &str = concat!(linux_card!(), package_folder!("/"));
/// The package folder in the RTOS.
pub const RTOS_PACKAGE_FOLDER: &str = concat!("C:\\", package_folder!("\\"));

/// The data folder at the card root, in the RTOS: the state files
/// (`capabilities::StateFile`), `REPORT.TXT`, the effects library in
/// `EFFECTS`, and the modules that a person adds in `MODULES`. No package
/// holds it, so a copy of a new package, by replace or by merge, keeps it.
/// The resident loader writes the state files through the camera's own file
/// system, never from Linux while the camera may write photos.
pub const DATA_FOLDER: &str = concat!("C:\\", data_folder!());
/// `DATA_FOLDER` on Linux.
pub const LINUX_DATA_DIRECTORY: &str = concat!(linux_card!(), data_folder!());
/// The folder of GRKit 1.2 and earlier on Linux: its state file and its
/// effects. INSTALL reads them once, when `DATA_FOLDER` has no state file or
/// no effects library.
pub const LINUX_LEGACY_DIRECTORY: &str = concat!(linux_card!(), "SCRIPT/PLUGINS");

/// The whole plan of a start, in RAM on Linux. SCAN writes it and INSTALL
/// reads it. The card gets only the plan header, which the loader reads, so a
/// start writes no large file to the card. RAM is empty at each power-on, so
/// a plan never reaches another start.
pub const LINUX_RAM_PLAN: &str = "/tmp/grkit-plan.bin";

/// The files of the package folder on the card.
pub const LOADER_FILE: &str = concat!(package_folder!("/"), "/LOADER.TTL");
/// The resident part of the loader, which `LOADER_FILE` includes after INSTALL
/// (`runtime::resident`).
pub const RESIDENT_FILE: &str = concat!(package_folder!("/"), "/RESIDENT.TTL");
pub const CORE_FILE: &str = concat!(package_folder!("/"), "/CORE.BIN");
pub const SCAN_FILE: &str = concat!(package_folder!("/"), "/SCAN.SH");
pub const INSTALL_FILE: &str = concat!(package_folder!("/"), "/INSTALL.SH");
/// The service on the camera's Linux, which a phone reaches over the camera's
/// Wi-Fi (crates/grkit-serve). It is the same file for each runtime build. A
/// runtime has it only when its build was given one. It starts from the card
/// and then runs from a copy in RAM, so it keeps no file on the card open.
pub const SERVE_FILE: &str = concat!(package_folder!("/"), "/SERVE.BIN");
/// The import library of the site (`crate::imports`). The camera does not read it.
pub const IMPORTS_FILE: &str = concat!(package_folder!("/"), "/IMPORTS.BIN");
/// The package list (`runtime/install_library.h`): the file name of each
/// effect and module of the package, one on each line. Each package has it,
/// also without effects, so a merge copy of a new package over an old one
/// gives the same result as a replace copy: the installer ignores the
/// package files that the list does not name.
pub const PACKAGE_LIST: &str = concat!(package_folder!("/"), "/PACKAGE.TXT");
/// The inbox of the package effects on the card. SCAN copies a new package
/// effect into the effects library, `GRKIT/EFFECTS`, where a person adds or
/// deletes effect files.
pub const EFFECTS_FOLDER: &str = concat!(package_folder!("/"), "/EFFECTS");
/// The recipe library of a package (`packaging::RECIPES_PACKAGE_PATH`).
pub const RECIPES_PACKAGE_FILE: &str = concat!(package_folder!("/"), "/RECIPES.BIN");
/// The data folder on the card, as a package path names it.
pub const DATA_FOLDER_PATH: &str = data_folder!();
/// The modules of a package (`grm`).
pub const MODULES_FOLDER: &str = concat!(package_folder!("/"), "/MODULES");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_spelling_names_the_same_folder() {
        assert_eq!(LINUX_DIRECTORY, format!("/media/storage1/{PACKAGE_FOLDER}"));
        assert_eq!(
            RTOS_PACKAGE_FOLDER,
            format!("C:\\{}", PACKAGE_FOLDER.replace('/', "\\"))
        );
        assert_eq!(
            LINUX_DATA_DIRECTORY.strip_prefix("/media/storage1/"),
            DATA_FOLDER.strip_prefix("C:\\")
        );
    }
}
