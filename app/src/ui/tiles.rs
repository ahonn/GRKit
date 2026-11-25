//! Square pictures at the start of rows and in grids: a Look's cover, a
//! system glyph on a tinted tile, and a module's pixel icon.

use grkit_core::cover::Cover;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::token;

/// The film box of a cover: the camera draws it in rows 3 to 37 of its 40,
/// with clear rows above and below.
const BOX_TOP: f32 = 3.0 / 40.0;
const BOX_HEIGHT: f32 = 34.0 / 40.0;

/// The size of a cover thumbnail `size` points wide: a film cover shows only
/// its box, a label tile its whole square.
pub fn cover_thumb_size(cover: &Cover, size: f32) -> (f32, f32) {
    match cover {
        Cover::Film(_) => (size, size * BOX_HEIGHT),
        Cover::Label(_) => (size, size),
    }
}

/// A Look's cover as the camera draws it, `size` points wide, cropped to the
/// film box and rounded a little. Faded while the Look is not in use.
pub fn cover_thumb(cover: &Cover, size: f32, faded: bool) -> AnyElement {
    let (width, height) = cover_thumb_size(cover, size);
    let lift = match cover {
        Cover::Film(_) => size * BOX_TOP,
        Cover::Label(_) => 0.0,
    };
    view()
        .w(px(width))
        .h(px(height))
        .flex_shrink(0.0)
        .rounded(px((size * 0.045).max(2.0)))
        .overflow_hidden()
        .opacity(if faded { 0.45 } else { 1.0 })
        .child(
            image(ImageSrc::svg_markup(&cover.svg()))
                .absolute()
                .left(px(0.0))
                .top(px(-lift))
                .w(px(size))
                .h(px(size)),
        )
        .into_element()
}

/// [`cover_thumb`] as wide as its container, such as a grid cell: the
/// height follows the width.
pub fn cover_fill(cover: &Cover, faded: bool) -> AnyElement {
    let (aspect, lift) = match cover {
        Cover::Film(_) => (1.0 / BOX_HEIGHT, BOX_TOP / BOX_HEIGHT),
        Cover::Label(_) => (1.0, 0.0),
    };
    view()
        .w_full()
        .aspect_ratio(aspect)
        .rounded(px(6.0))
        .overflow_hidden()
        .opacity(if faded { 0.45 } else { 1.0 })
        .child(
            image(ImageSrc::svg_markup(&cover.svg()))
                .absolute()
                .left(px(0.0))
                // A fraction of the box's height, as CSS resolves `top`.
                .top(relative(-lift))
                .w_full()
                .aspect_ratio(1.0),
        )
        .into_element()
}

/// The colors of an [`icon_tile`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileTone {
    /// The brand tint on its soft fill: a thing to act on, such as an update.
    Tint,
    /// White on the camera's dark tile: a GRKit setting, as the camera shows it.
    Camera,
}

/// Stroked glyphs on a 24-unit grid for [`icon_tile`], drawn in the tile's
/// color.
pub mod glyph {
    /// An arrow down into a circle: a software update.
    pub const DOWNLOAD: &str =
        r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v8M8.5 12.5L12 16l3.5-3.5"/>"#;
    /// A sheet of lines: a recipe.
    pub const RECIPE: &str = r#"<rect x="5" y="2.5" width="14" height="19" rx="2.5"/><path d="M8.5 8h7M8.5 12h7M8.5 16h4.5"/>"#;
    /// The power symbol: Enable at Start.
    pub const POWER: &str = r#"<path d="M12 3v8"/><path d="M7 6.3a8 8 0 1 0 10 0"/>"#;
    /// Two sliders: Modules.
    pub const SLIDERS: &str = r#"<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>"#;
    /// A plus: add.
    pub const PLUS: &str = r#"<path d="M12 5v14M5 12h14"/>"#;
    /// A camera: a page that needs the camera connected.
    pub const CAMERA: &str = r#"<path d="M3 8.5A2.5 2.5 0 0 1 5.5 6h2l1.5-2.2h6L16.5 6h2A2.5 2.5 0 0 1 21 8.5v9a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z"/><circle cx="12" cy="12.8" r="3.6"/>"#;
    /// A triangle with an exclamation mark: something could not be read.
    pub const WARNING: &str = r#"<path d="M10.3 4.2a2 2 0 0 1 3.4 0l7.6 13.2a2 2 0 0 1-1.7 3H4.4a2 2 0 0 1-1.7-3z"/><path d="M12 9.5v4.5M12 17.2h.01"/>"#;
    /// A picture: Shutdown Screen.
    pub const PICTURE: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M21 16l-5-5-8 8"/>"#;
}

/// A rounded square of `size` points holding a stroked `glyph` (one of
/// [`glyph`]) in the colors of `tone`.
pub fn icon_tile(glyph: &str, size: f32, tone: TileTone) -> AnyElement {
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{glyph}</svg>"#
    );
    let fill = move || match tone {
        TileTone::Tint => token("tint-soft"),
        // A step lighter in dark mode, so the tile stands off a dark card.
        TileTone::Camera if dark() => rgb(0x3A3A3C).into(),
        TileTone::Camera => rgb(0x2C2C2E).into(),
    };
    let ink: AdaptiveColor = match tone {
        TileTone::Tint => token("tint"),
        TileTone::Camera => rgb(0xFFFFFF).into(),
    };
    view()
        .size(px(size))
        .flex_shrink(0.0)
        .rounded(px(size * 0.25))
        .items_center()
        .justify_center()
        .bg(fill)
        .text_color(ink)
        .child(svg(ImageSrc::svg_markup(&markup)).size(px(size * 0.56)))
        .into_element()
}

/// The tile of a module, `size` points square: its pixel icon from the
/// camera (`domain::modules::pixel_icon_svg`), else the initial of its title
/// on the camera's dark tile.
pub fn module_icon_tile(icon: Option<&str>, initial: &str, size: f32, faded: bool) -> AnyElement {
    let tile = view()
        .size(px(size))
        .flex_shrink(0.0)
        .rounded(px(size * 0.22))
        .overflow_hidden()
        .bg(rgb(0x353536))
        .items_center()
        .justify_center()
        .opacity(if faded { 0.45 } else { 1.0 });
    match icon {
        Some(markup) => tile
            .child(image(ImageSrc::svg_markup(markup)).size(px(size)))
            .into_element(),
        // Drawn on a fixed tile, as a cover is.
        None => tile
            .child(
                crate::ui::ty::text(initial.to_owned())
                    .allow_font_scaling(false)
                    .text_size(px(size * 0.45))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xffffff)),
            )
            .into_element(),
    }
}

/// The stacked cards of the camera's Recipes module, small, on a dark tile:
/// the mark of a recipe in the corner of its cover.
const RECIPE_BADGE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 18 18" fill="none"><rect width="18" height="18" rx="5" fill="#353536"/><rect x="7.5" y="3" width="7" height="6" rx="0.5" stroke="#D2D2D2"/><rect x="5.5" y="5" width="7" height="6" rx="0.5" fill="#111111" stroke="#D2D2D2"/><rect x="3.5" y="7" width="7" height="6" rx="0.5" fill="#111111" stroke="#D2D2D2"/></svg>"##;

/// The side of [`recipe_badge`], and the room it takes past the tile.
const BADGE: f32 = 14.0;
const BADGE_OUTSET: f32 = 4.0;

/// A recipe's `tile` (its cover, Image Control icon or name), `size` points
/// wide, with the stacked cards of the Recipes module in its lower right
/// corner, so that a recipe does not read as its Film Look. The mark keeps
/// a ring in the card's color, so that it reads on any cover.
pub fn recipe_badge(tile: AnyElement, size: f32, faded: bool) -> AnyElement {
    view()
        .w(px(size + BADGE_OUTSET))
        .h(px(size + 2.0))
        .flex_shrink(0.0)
        .justify_center()
        .child(tile)
        .child(
            view()
                .absolute()
                .right(px(0.0))
                .bottom(px(0.0))
                .p(px(1.5))
                .rounded(px(5.5))
                .bg(move || token("list-card"))
                .opacity(if faded { 0.45 } else { 1.0 })
                .child(image(ImageSrc::svg_markup(RECIPE_BADGE)).size(px(BADGE))),
        )
        .into_element()
}
