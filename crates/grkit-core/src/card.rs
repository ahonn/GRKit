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
