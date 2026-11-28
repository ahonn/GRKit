//! Shared phone chrome: grouped rows, camera art, primary buttons.

mod chrome;

pub use chrome::*;

mod image_control;
mod image_control_glyphs;
pub use image_control::image_control_tile;

mod loading;
pub use loading::{cover_row_skeleton, hero_skeleton, skeleton_bar};

// The shared components of the redesign. Pages move to them one by one,
// so some are not used yet.
#[allow(dead_code)]
mod list;
#[allow(unused_imports)]
pub use list::{
    ListRow, ListSection, ROW_TWO_LINES, SECTION_RADIUS, list_note, list_row, list_section,
};

#[allow(dead_code)]
mod tiles;
#[allow(unused_imports)]
pub use tiles::{
    TileTone, cover_fill, cover_thumb, cover_thumb_size, glyph, icon_tile, module_icon_tile,
    recipe_badge,
};

#[allow(dead_code)]
mod button;
#[allow(unused_imports)]
pub use button::{Button, ButtonKind, ButtonSize, button, text_link};

#[allow(dead_code)]
mod chip;
#[allow(unused_imports)]
pub use chip::{ChipTone, chip, row_chip, status_line};

#[allow(dead_code)]
mod cover_grid;
#[allow(unused_imports)]
pub use cover_grid::{CaptionTone, CoverCell, cover_cell, cover_grid, cover_grid_of};

#[allow(dead_code)]
mod media;
#[allow(unused_imports)]
pub use media::{DetailHeader, HERO_HEIGHT, PhotoHero, detail_header, overlay_button, photo_hero};

#[allow(dead_code)]
mod cards;
#[allow(unused_imports)]
pub use cards::{FeatureCard, NoticeCard, feature_card, notice_card};

#[allow(dead_code)]
mod reorder;
#[allow(unused_imports)]
pub use reorder::{HandleDrag, RowGestures, check_circle, drag_handle, list, reorder_list};

mod system;
pub use system::{Choice, Choices, held_view, menu_row, system_segmented, system_switch};

mod empty_state;
pub use empty_state::empty_state;

/// Room under scrolled content for the floating tab bar.
pub const TAB_BAR_CLEARANCE: f32 = 96.0;

pub mod setting_icons;
pub mod ty;

mod toast;
pub use toast::{Toasts, toast_layer};

#[cfg(test)]
mod tests;
