//! Package-time module visibility.
//!
//! `SCRIPT/PLUGINS/MODULES.TXT` names the ADJ modules that the camera menu hides,
//! separated by ASCII whitespace. The installer parses it with the same rules
//! (`runtime/visibility.h`). A package without the file hides nothing.
//! Image Control is not listed: it shows exactly when the package has effects.

use anyhow::{bail, ensure, Result};

/// The path of the file in the SD tree.
pub const PATH: &str = "SCRIPT/PLUGINS/MODULES.TXT";
/// The largest file that the installer reads.
pub const MAX_FILE_SIZE: usize = 256;
/// Modules that a package can hide, in menu order. Name n is module n + 1 of
/// `runtime/modules.h`, after Image Control.
pub const NAMES: [&str; 3] = ["framing", "guides", "date-stamp"];

/// A set of hidden modules. Bit n hides module n of `runtime/modules.h`, as the
/// installer stores it in the private header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hidden(u32);

impl Hidden {
    /// Hides the named modules. Unknown and repeated names are errors.
    pub fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self> {
        let mut mask = 0;
        for name in names {
            let Some(index) = NAMES.iter().position(|known| *known == name) else {
                bail!(
                    "Unknown module {name:?}. Expected one of: {}",
                    NAMES.join(", ")
                );
            };
            let bit = 2 << index;
            ensure!(mask & bit == 0, "Module listed twice: {name}");
            mask |= bit;
        }
        Ok(Self(mask))
    }

    /// Reads the contents of `MODULES.TXT`.
    pub fn parse(file: &[u8]) -> Result<Self> {
        ensure!(
            file.len() <= MAX_FILE_SIZE,
            "MODULES.TXT exceeds {MAX_FILE_SIZE} bytes"
        );
        let names = file
            .split(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            .filter(|token| !token.is_empty())
            .map(|token| std::str::from_utf8(token).unwrap_or("\u{fffd}"));
        Self::from_names(names)
    }

    /// The contents of `MODULES.TXT`, one name for each line, or `None` when
    /// nothing is hidden. A package then omits the file.
    pub fn file(self) -> Option<Vec<u8>> {
        (!self.is_empty()).then(|| {
            self.names()
                .iter()
                .map(|name| format!("{name}\n"))
                .collect::<String>()
                .into_bytes()
        })
    }

    /// The hidden module names, in menu order.
    pub fn names(self) -> Vec<&'static str> {
        NAMES
            .iter()
            .enumerate()
            .filter(|(index, _)| self.0 & 2 << index != 0)
            .map(|(_, name)| *name)
            .collect()
    }

    pub fn mask(self) -> u32 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True when the camera menu would show no module for this many effects.
    pub fn hides_everything(self, effects: usize) -> bool {
        effects == 0 && self.names().len() == NAMES.len()
    }
}
