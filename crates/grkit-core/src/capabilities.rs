//! The capabilities of the core: public settings `CORE.<KEY>` that modules
//! name in `uses` (`runtime/sdk/grkit.h`).
//!
//! The core owns the state of each capability in the private header, and its
//! hooks read that state. A module only reads or writes the setting. The
//! installer applies the patches of a capability only when a loaded module
//! uses its setting. The resident loader saves the value as the state key
//! `CORE.<KEY>`, and restores it only while a loaded module uses it.

use crate::abi::{HOST_FRAME_RATIO, HOST_GRAIN, HOST_GRAIN_SIZE};

/// An extension point: the firmware changes that a group of features needs.
/// The installer applies the patches of a point only when a loaded module
/// needs it. The hooks of each family in `grkit_build::target` give their points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    /// The private ADJ entry, its list, text, and icons.
    MenuPage,
    /// Image Control: private processing records, the effect tag, and the
    /// native Image Control pages.
    Tuning,
    /// Drawing over the still live view.
    ViewfinderOverlay,
    /// The capture aspect and the recorded picture geometry.
    CaptureAspect,
    /// Changes to the JPEG planes of a recorded picture.
    PictureEncode,
    /// The native grain stage of recorded pictures.
    PipelineGrain,
    /// The grain size: the native size on GR IV, and the noise fill of the
    /// grain stage on GR III.
    PipelineGrainSize,
}

impl Point {
    pub const ALL: [Point; 7] = [
        Point::MenuPage,
        Point::Tuning,
        Point::ViewfinderOverlay,
        Point::CaptureAspect,
        Point::PictureEncode,
        Point::PipelineGrain,
        Point::PipelineGrainSize,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Point::MenuPage => "menu.page",
            Point::Tuning => "tuning",
            Point::ViewfinderOverlay => "viewfinder.overlay",
            Point::CaptureAspect => "capture.aspect",
            Point::PictureEncode => "picture.encode",
            Point::PipelineGrain => "pipeline.grain",
            Point::PipelineGrainSize => "pipeline.grain-size",
        }
    }

    pub fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// The module ID of the core settings.
pub const MODULE: &str = "CORE";
/// The handle of capability n is `HANDLE_BASE + n`. It is above the handles of
/// module settings (`runtime::MODULE_SETTINGS_MAX`).
pub const HANDLE_BASE: u32 = 0x100;

/// Where the core keeps the value of a capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// A header word at this byte offset.
    Word(usize),
    /// The active Image Control record. The state file keeps its ImageTone
    /// code, so the value stays with its effect when the effects change.
    Effect,
    /// No value of its own: the page rows are the camera recipes
    /// (`runtime/recipes.h`), which the state file keeps as recipe entries
    /// (`runtime::RECIPE_KEY_BASE`). It has no state key.
    Recipes,
}

pub struct Capability {
    pub key: &'static str,
    /// The number of values: 0 to `values - 1`. 0 for a capability whose
    /// number of values the installer sets (`CORE.TONE`: the effects + 1).
    pub values: u32,
    pub state: State,
    /// The extension points whose patches apply the value.
    pub points: &'static [Point],
}

impl Capability {
    /// `CORE.<KEY>`.
    pub fn name(&self) -> String {
        format!("{MODULE}.{}", self.key)
    }

    pub fn point_mask(&self) -> u32 {
        self.points.iter().fold(0, |mask, point| mask | point.bit())
    }
}

/// The capabilities in handle order.
pub const CAPABILITIES: [Capability; 5] = [
    Capability {
        // The effect of still pictures and the live view: 0 Off, then effect
        // n - 1 of the installed effects (runtime/tone.h). The core gives
        // the rows of its page: the effect names, icons, and Fn adjustments.
        // Without a module that uses it, the installer converts no effect.
        key: "TONE",
        values: 0,
        state: State::Effect,
        points: &[Point::Tuning, Point::PictureEncode],
    },
    Capability {
        // The aspect of still pictures (runtime/aspect.h): 0 native 3:2, then
        // XPan 65:24, 16:9, 4:3, 2.35:1, 1:1, 5:4, 7:6. Each target offers some of
        // them. The value comes from the native aspect and the crop, and the
        // word holds the crop. The overlay keeps the live view of a crop in
        // sync with the capture.
        key: "ASPECT",
        values: 8,
        state: State::Word(HOST_FRAME_RATIO),
        points: &[Point::ViewfinderOverlay, Point::CaptureAspect],
    },
    Capability {
        // 0 Off, 1 weak, 2 medium, 3 strong, 4 extra strong, in the native
        // grain stage of recorded pictures. Extra strong is strong with
        // larger grain tables (runtime/picture.S: grain_amplitude).
        key: "GRAIN",
        values: 5,
        state: State::Word(HOST_GRAIN),
        points: &[Point::PipelineGrain],
    },
    Capability {
        // The grain size: 0 Auto, which follows the ISO of the shot, then 1
        // small, 2 medium, 3 large (runtime/picture.S).
        key: "GRAIN_SIZE",
        values: 4,
        state: State::Word(HOST_GRAIN_SIZE),
        points: &[Point::PipelineGrainSize],
    },
    Capability {
        // The camera recipes (runtime/recipes.h). The core gives the rows of
        // its page: undo, the recipes, and save. A recipe sets the other
        // capabilities only while a module uses them, so it needs no point.
        key: "RECIPE",
        values: 0,
        state: State::Recipes,
        points: &[],
    },
];

/// A state file of the resident loader in `card::DATA_FOLDER`. The loader
/// restores the files in the order of `ALL`, so the settings of the main file
/// apply first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateFile {
    /// The values of the capabilities and the settings of the modules, and
    /// the Fn adjustments of the effects.
    Main,
}

impl StateFile {
    pub const ALL: [StateFile; 1] = [StateFile::Main];

    /// The file name without its extension. The file is `<stem>.TXT`, and the
    /// loader writes `<stem>.TMP` before it replaces the file.
    pub fn stem(self) -> &'static str {
        match self {
            Self::Main => "STATE",
        }
    }
}

/// A state key of a capability.
pub struct StateEntry {
    /// `CORE.<KEY>`.
    pub name: String,
    /// The header word that holds the value, or `None` for the effect, which
    /// the file keeps as the ImageTone code of the record.
    pub word: Option<usize>,
    /// The largest value that the loader restores, or 0xffff for the effect.
    pub max: u32,
    /// The bit of the capability in `GRKIT_HOST_CAPABILITIES`.
    pub bit: u32,
    pub file: StateFile,
}

/// The state keys of the capabilities, in handle order. The effect is first.
/// `CORE.RECIPE` has none: the capabilities with a key come first, so key n
/// is capability n.
pub fn state_keys() -> Vec<StateEntry> {
    CAPABILITIES
        .iter()
        .enumerate()
        .filter_map(|(index, capability)| {
            let (word, max) = match capability.state {
                State::Word(offset) => (Some(offset), capability.values - 1),
                State::Effect => (None, 0xffff),
                State::Recipes => return None,
            };
            Some(StateEntry {
                name: capability.name(),
                word,
                max,
                bit: index as u32,
                file: StateFile::Main,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_follow_the_module_settings() {
        assert!(HANDLE_BASE as usize >= crate::runtime::MODULE_SETTINGS_MAX);
        for (i, capability) in CAPABILITIES.iter().enumerate() {
            assert!(CAPABILITIES[..i].iter().all(|c| c.key != capability.key));
            match capability.state {
                State::Word(_) => assert!(capability.values >= 2),
                State::Effect | State::Recipes => assert_eq!(capability.values, 0),
            }
        }
        // The resident loader keeps the effect in key 0.
        assert_eq!(CAPABILITIES[0].state, State::Effect);
        // The capabilities without a state key come last, so the state key
        // of index n is capability n (CORE_STATE_IDS in runtime/installer.c).
        let keyed = state_keys().len();
        assert!(CAPABILITIES[keyed..]
            .iter()
            .all(|capability| capability.state == State::Recipes));
    }
}
