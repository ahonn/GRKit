//! The GR III family: GR III and GR IIIx, which run the same RTOS code. Its
//! hooks, in installation order: the shared hooks (`super::common`) and its
//! own. Each build gives the site of each hook.

use super::common::{self, join, len};
use super::Family;
use crate::components::{
    b, bl, bl_shared, HookSpec,
    Part::{Framing, Menu, Processing},
    Point::{
        CaptureAspect, CaptureCrop, MenuPage, ModeDial, PictureCanvas, PipelineGrain,
        PipelineGrainSize, Tuning,
    },
};

pub const FAMILY: Family = Family {
    name: "gr3",
    hooks: HOOKS,
    processing_source: Some(include_str!("../../../../runtime/processing.S")),
    scale_one_column: 4,
};

const HOOKS: &[HookSpec] = &join::<{ len(GROUPS) }>(GROUPS);

const GROUPS: &[&[HookSpec]] = &[
    PROCESSING,
    common::ADJ,
    ICONS,
    common::IMAGE_CONTROL_PAGES,
    common::MENU,
    common::KEYBOARD,
    common::VIEWFINDER,
    common::PICTURE,
    common::CROP,
    common::SCALE,
    CROP,
    common::DEVELOP_TONE,
    common::DEVELOP_ASPECT,
    GRAIN,
    TONING,
    DIAL,
    CANVAS,
];

/// Image Control processing (`runtime/processing.S`): the BL to the parameter
/// normalizer, the type-table loads, the last packet field store, the
/// custom-curve flag, and the BL to the curve getter.
const PROCESSING: &[HookSpec] = &[
    // The shot ring records the capture crop and the border even without
    // Film Look.
    bl_shared(
        "module_parameters",
        Processing,
        Tuning,
        &[CaptureAspect, PictureCanvas],
    ),
    b("module_processing", Processing, Tuning),
    b("module_packet", Processing, Tuning),
    b("module_gamma_flag", Processing, Tuning),
    bl("module_gamma_curves", Processing, Tuning),
];

/// GetImageCrop after its first instruction: GR III maps only its native
/// crop factors to the maps of their crop step, and a crop factor of
/// CORE.CROP takes the maps of its step (`runtime/framing.S`).
///
/// The four BLs to the live-view crop factor before the live view scales it
/// by 8192 for its zoom: a crop that the live view shows takes its own
/// factor there (`runtime/framing.S: crop_live`), while the camera keeps its
/// own crop factor. Then the entry of the function that makes the window
/// of the live view from its zoom (`runtime/framing.S: crop_window`).
const CROP: &[HookSpec] = &[
    b("crop_image_crop", Framing, CaptureCrop),
    bl("crop_live_0", Framing, CaptureCrop),
    bl("crop_live_1", Framing, CaptureCrop),
    bl("crop_live_2", Framing, CaptureCrop),
    bl("crop_live_3", Framing, CaptureCrop),
    b("crop_window", Framing, CaptureCrop),
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
    // Film Look needs the gate for the Shading and Clarity of its effects.
    bl_shared("grain_gate", Framing, PipelineGrain, &[Tuning]),
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

/// The mode dial (`runtime/mode_dial.h`): the entry of the apply of a dial
/// position, which both App events of the dial reach.
const DIAL: &[HookSpec] = &[b("menu_dial", Menu, ModeDial)];

/// The canvas of CORE.BORDER (`runtime/canvas_shot.h`): the BL to the R2Y
/// stripe send of StillMake, and the instruction after the resize of
/// BackResize.
const CANVAS: &[HookSpec] = &[
    bl("canvas_r2y", Framing, PictureCanvas),
    b("canvas_resize", Framing, PictureCanvas),
];
