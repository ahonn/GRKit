//! The GR III family: GR III and GR IIIx, which run the same RTOS code. Its
//! hooks, in installation order: the shared hooks (`super::common`) and its
//! own. Each build gives the site of each hook.

use super::common::{self, join, len};
use super::Family;
use crate::components::{
    b, bl, HookSpec,
    Part::{Framing, Menu, Processing},
    Point::{MenuPage, PipelineGrain, PipelineGrainSize, Tuning},
};

pub const FAMILY: Family = Family {
    name: "gr3",
    hooks: HOOKS,
    processing_source: Some(include_str!("../../../../runtime/processing.S")),
};

const HOOKS: &[HookSpec] = &join::<{ len(GROUPS) }>(GROUPS);

const GROUPS: &[&[HookSpec]] = &[
    PROCESSING,
    common::ADJ,
    ICONS,
    common::IMAGE_CONTROL_PAGES,
    common::MENU,
    common::VIEWFINDER,
    common::PICTURE,
    GRAIN,
    TONING,
];

/// Image Control processing (`runtime/processing.S`): the BL to the parameter
/// normalizer, the type-table loads, the last packet field store, the
/// custom-curve flag, and the BL to the curve getter.
const PROCESSING: &[HookSpec] = &[
    bl("module_parameters", Processing, Tuning),
    b("module_processing", Processing, Tuning),
    b("module_packet", Processing, Tuning),
    b("module_gamma_flag", Processing, Tuning),
    bl("module_gamma_curves", Processing, Tuning),
];

/// The icon width, height, and limit checks, and the descriptor load.
const ICONS: &[HookSpec] = &[
    b("menu_icon_width", Menu, MenuPage),
    b("menu_icon_height", Menu, MenuPage),
    b("menu_icon_guard", Menu, MenuPage),
    b("menu_icon_descriptor", Menu, MenuPage),
];

/// DigitalFilterProcess SetEffectParameter: the Particle load, then the BL to
/// the stage gate. The granulation: the BL to its noise fill, which gives
/// CORE.GRAIN_SIZE, and the first test of its amplitude choice, which gives
/// Extra Strong.
const GRAIN: &[HookSpec] = &[
    bl("grain_particle", Framing, PipelineGrain),
    bl("grain_gate", Framing, PipelineGrain),
    bl("grain_fill", Framing, PipelineGrainSize),
    b("grain_amplitude", Framing, PipelineGrain),
];

/// Toning of a native look on the parameter page (`runtime/toning.h`): the
/// Image Control model getter and setter of the Retro Toning row (kind 9), and
/// the BL to its settings getter in the row text.
const TONING: &[HookSpec] = &[
    b("menu_parameter_get_9", Menu, Tuning),
    b("menu_parameter_set_9", Menu, Tuning),
    bl("menu_parameter_row_9", Menu, Tuning),
];
