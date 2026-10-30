//! The GR IV family: each GR IV build, including GR IV HDF and GR IV
//! Monochrome. Its hooks, in installation order: the shared hooks
//! (`super::common`) and its own. Each build gives the site of each hook.
//!
//! On the parameter page, the model getters and setters and the row texts of
//! shading and clarity are those of row kinds 14 and 15. The picture geometry
//! hooks are in ImageResolutionTranslatorKb636 (the GR IV sizes); the Kb588
//! class holds the GR III sizes and is not used.

use super::common::{self, join, len};
use super::Family;
use crate::components::{
    b, bl, HookSpec,
    Part::{Framing, Menu, Processing},
    Point::{CaptureAspect, MenuPage, PipelineGrain, PipelineGrainSize, Tuning},
};

pub const FAMILY: Family = Family {
    name: "gr4",
    hooks: HOOKS,
    processing_source: Some(include_str!("../../../../runtime/targets/gr4-processing.S")),
};

const HOOKS: &[HookSpec] = &join::<{ len(GROUPS) }>(GROUPS);

const GROUPS: &[&[HookSpec]] = &[
    PROCESSING,
    common::ADJ,
    common::IMAGE_CONTROL_PAGES,
    ICONS,
    common::MENU,
    common::KEYBOARD,
    common::VIEWFINDER,
    LIVE_VIEW,
    common::PICTURE,
    GRAIN,
];

/// Image Control processing (`runtime/targets/gr4-processing.S`): the BL to
/// the parameter normalizer, `mov r1, sl` before the type-table getters, the
/// record and MultiAxial set stores, the last packet field store, the
/// custom-curve flag, and the BL to the curve getter.
const PROCESSING: &[HookSpec] = &[
    bl("module_parameters", Processing, Tuning),
    bl("module_processing", Processing, Tuning),
    bl("module_record", Processing, Tuning),
    bl("module_multiaxial", Processing, Tuning),
    bl("module_packet", Processing, Tuning),
    bl("module_gamma_flag", Processing, Tuning),
    bl("module_gamma_curves", Processing, Tuning),
];

/// Icon lookup entry, then the limit check and the table read of the icon
/// draw.
const ICONS: &[HookSpec] = &[
    b("menu_icon_lookup", Menu, MenuPage),
    b("menu_icon_limit", Menu, MenuPage),
    b("menu_icon_entry", Menu, MenuPage),
];

/// GR IV has a live-view mode for each capture aspect, so instead of the GR
/// III patches the capture aspect getter BLs of the live-view mode function
/// and of the live-view mode check keep the live view in 3:2 while XPan uses
/// capture aspect id 2.
const LIVE_VIEW: &[HookSpec] = &[
    bl("framing_live_mode", Framing, CaptureAspect),
    bl("framing_live_check", Framing, CaptureAspect),
];

/// DigitalFilterProcess SetEffectParameter: the grain on, strength, and size
/// loads, then the BL to the stage gate. The size load gives CORE.GRAIN_SIZE.
/// The grain tables: the `mov r0, #0x200` after the load of their luma
/// amplitude, which gives Extra Strong.
const GRAIN: &[HookSpec] = &[
    bl("grain_enable", Framing, PipelineGrain),
    bl("grain_strength", Framing, PipelineGrain),
    bl("grain_size", Framing, PipelineGrainSize),
    bl("grain_gate", Framing, PipelineGrain),
    bl("grain_amplitude", Framing, PipelineGrain),
];
