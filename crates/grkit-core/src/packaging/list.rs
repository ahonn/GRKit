//! The package list (`PACKAGE_LIST`) and the files that a package owns.

use super::*;

/// The contents of `PACKAGE_LIST`: the file names in name order, one on
/// each line, then each left-out effect after `-`.
pub(crate) fn package_list<'a>(
    names: impl IntoIterator<Item = &'a String>,
    left_out: &BTreeSet<String>,
) -> Vec<u8> {
    let mut names: Vec<&String> = names.into_iter().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| format!("{name}\n"))
        .chain(left_out.iter().map(|name| format!("-{name}\n")))
        .collect::<String>()
        .into_bytes()
}

/// `PACKAGE_LIST` as the runtime reads it (`runtime/install_library.h`): names separated
/// by ASCII whitespace, and each left-out effect after `-`. The runtime compares names
/// without case, so the names here are upper case.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageList {
    /// The module and effect files of the package.
    pub names: BTreeSet<String>,
    /// The effect files that the person did not choose (`EffectSet`).
    pub left_out: BTreeSet<String>,
}

impl PackageList {
    pub fn parse(bytes: &[u8]) -> Self {
        let mut list = Self::default();
        for word in String::from_utf8_lossy(bytes).split_ascii_whitespace() {
            let word = word.to_ascii_uppercase();
            match word.strip_prefix('-') {
                Some(name) if !name.is_empty() => {
                    list.left_out.insert(name.to_owned());
                }
                Some(_) => {}
                None => {
                    list.names.insert(word);
                }
            }
        }
        list
    }

    /// The bytes that a package writes (`package_list`).
    pub fn to_bytes(&self) -> Vec<u8> {
        package_list(&self.names, &self.left_out)
    }

    /// The module IDs that the list names.
    pub fn modules(&self) -> impl Iterator<Item = &str> {
        self.names
            .iter()
            .filter_map(|name| name.strip_suffix(".GRM"))
    }

    /// The effect files that the list names.
    pub fn effects(&self) -> impl Iterator<Item = &str> {
        self.names
            .iter()
            .filter(|name| name.ends_with(".GRF"))
            .map(String::as_str)
    }

    /// This list with the Looks of `card`, the listed and the left-out ones, in place of
    /// its own: an update keeps the Looks of the card.
    pub fn with_looks_of(mut self, card: &PackageList) -> Self {
        self.names.retain(|name| !name.ends_with(".GRF"));
        self.names.extend(card.effects().map(str::to_owned));
        self.left_out = card.left_out.clone();
        self
    }
}

/// Whether the camera puts the card file `path` there with each package: the list, the
/// release evidence, the runtime files, the modules, and the package effects. An update
/// removes only these when the new package does not have them; other files of the package
/// folder, such as the import library, stay.
pub fn package_owned(path: &str) -> bool {
    if path == PACKAGE_LIST
        || path == crate::release::CARD_STATEMENT
        || path == crate::release::CARD_SIGNATURE
        || is_runtime_path(path)
    {
        return true;
    }
    let in_folder = |folder: &str, extension: &str| {
        path.strip_prefix(folder)
            .and_then(|name| name.strip_prefix('/'))
            .is_some_and(|name| !name.contains('/') && name.ends_with(extension))
    };
    let core = path
        .strip_prefix(card::PACKAGE_FOLDER)
        .and_then(|rest| rest.strip_prefix("/CORE"))
        .is_some_and(|rest| rest.ends_with(".BIN") && !rest.contains('/'));
    in_folder(card::MODULES_FOLDER, ".GRM") || in_effect_inbox(path) || core
}

/// Whether `path` is a Look file in the inbox of the package effects (`EFFECTS_FOLDER`).
pub fn in_effect_inbox(path: &str) -> bool {
    path.strip_prefix(EFFECTS_FOLDER)
        .and_then(|name| name.strip_prefix('/'))
        .is_some_and(|name| !name.contains('/') && name.ends_with(".GRF"))
}

/// A package filename of an effect: 1 to 8 uppercase letters, digits, or `_`, then `.GRF`.
pub(super) fn effect_file_name(name: &str) -> bool {
    name.strip_suffix(".GRF").is_some_and(|stem| {
        (1..=8).contains(&stem.len())
            && stem
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    })
}

pub(super) fn is_effect_path(name: &str) -> bool {
    name.strip_prefix("SD_ROOT/SCRIPT/GRKIT/EFFECTS/")
        .is_some_and(effect_file_name)
}
