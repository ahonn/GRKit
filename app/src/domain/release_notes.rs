//! Which release notes the Software Update page shows. The notes come with
//! the release index, newest version first, in English.

pub use grkit_core::release_index::VersionNotes;
use grkit_core::release_index::semver;

/// Items shown before "Show All".
pub const FOLDED_ITEMS: usize = 3;

/// The items of `version`, empty when the index has none.
pub fn items_of(notes: &[VersionNotes], version: &str) -> Vec<String> {
    notes
        .iter()
        .find(|n| n.version == version)
        .map(|n| n.items.clone())
        .unwrap_or_default()
}

/// Every version with notes, newest first.
pub fn history(notes: &[VersionNotes]) -> Vec<VersionNotes> {
    let mut all: Vec<VersionNotes> = notes
        .iter()
        .filter(|n| !n.items.is_empty())
        .cloned()
        .collect();
    all.sort_by_key(|n| std::cmp::Reverse(semver(&n.version)));
    all
}

/// The first `limit` items, and whether some are left out.
pub fn fold(items: &[String], limit: usize) -> (&[String], bool) {
    (&items[..items.len().min(limit)], items.len() > limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(version: &str, count: usize) -> VersionNotes {
        VersionNotes {
            version: version.into(),
            items: (0..count).map(|i| format!("{version} #{i}")).collect(),
        }
    }

    #[test]
    fn history_is_newest_first_without_empty_versions() {
        let all = [
            notes("1.6.2", 1),
            notes("1.7.0", 1),
            notes("1.6.10", 1),
            notes("1.6.9", 0),
        ];
        let versions: Vec<_> = history(&all).into_iter().map(|n| n.version).collect();
        assert_eq!(versions, ["1.7.0", "1.6.10", "1.6.2"]);
        assert_eq!(items_of(&all, "1.6.2"), ["1.6.2 #0"]);
        assert!(items_of(&all, "1.5.0").is_empty());
    }

    #[test]
    fn fold_keeps_the_first_items() {
        let items = notes("1.7.0", 5).items;
        assert_eq!(fold(&items, FOLDED_ITEMS), (&items[..3], true));
        assert_eq!(fold(&items[..3], FOLDED_ITEMS), (&items[..3], false));
    }
}
