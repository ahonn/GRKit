//! The app's text sizes, named after the iOS text styles at their default
//! size. NSH scales them with the person's text size. The brand mark and the
//! labels drawn on cover tiles keep sizes of their own.

use nsh_core::prelude::*;

/// The largest text size the layouts hold: rows, tiles and bars break into
/// single-character lines above it.
const MAX_TEXT_SCALE: f32 = 1.5;

/// Text that follows the person's text size up to [`MAX_TEXT_SCALE`].
pub fn text(content: impl IntoReactive<SharedString>) -> Text {
    nsh_core::prelude::text(content).max_font_size_multiplier(MAX_TEXT_SCALE)
}

pub const TITLE2: f32 = 22.0;
pub const TITLE3: f32 = 20.0;
pub const HEADLINE: f32 = 17.0;
pub const BODY: f32 = 17.0;
pub const CALLOUT: f32 = 16.0;
pub const SUBHEADLINE: f32 = 15.0;
pub const FOOTNOTE: f32 = 13.0;
pub const CAPTION: f32 = 12.0;

/// The title of a filled button, at the platform's size.
pub const BUTTON: f32 = if cfg!(target_os = "android") {
    CALLOUT
} else {
    HEADLINE
};
