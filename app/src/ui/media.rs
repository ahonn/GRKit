//! Photos that carry a Look: the large card of what the camera uses, the
//! header of a Look or recipe page, and the capsule button over a photo.

use grkit_core::cover::Cover;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;
use crate::ui::{cover_thumb, token};

/// The height of a [`photo_hero`] on a phone.
pub const HERO_HEIGHT: f32 = 232.0;

/// The photo of a [`detail_header`], and how far the page's sheet and the
/// cover reach over it.
pub const HEADER_PHOTO_HEIGHT: f32 = 330.0;
pub const HEADER_SHEET_OVERLAP: f32 = 30.0;
pub const HEADER_COVER_SIZE: f32 = 88.0;
/// The cover's top: it sits across the sheet's edge.
const HEADER_COVER_TOP: f32 = HEADER_PHOTO_HEIGHT - 68.0;

/// The fill under a photo that has not come yet, and under a card without
/// one.
const DARK_FILL: u32 = 0x2C2C2E;

/// What a [`photo_hero`] shows at its start, beside its title.
enum HeroMark {
    None,
    Cover(Cover),
    /// Neither a recipe nor a Look is in use.
    Empty,
}

/// The large card of what the camera uses: a photo with the Look, its
/// cover, and a title over the photo's foot. Build it with [`photo_hero`].
pub struct PhotoHero {
    kicker: String,
    title: String,
    photo: Option<ImageSrc>,
    center: Option<AnyElement>,
    mark: HeroMark,
    open: Option<Box<dyn Fn()>>,
    test_id: Option<String>,
}

/// A card titled `title`, with `kicker` above the title (Film Look · In Use).
pub fn photo_hero(kicker: impl Into<String>, title: impl Into<String>) -> PhotoHero {
    PhotoHero {
        kicker: kicker.into(),
        title: title.into(),
        photo: None,
        center: None,
        mark: HeroMark::None,
        open: None,
        test_id: None,
    }
}

impl PhotoHero {
    /// The photo with the Look, across the card.
    pub fn photo(mut self, photo: ImageSrc) -> Self {
        self.photo = Some(photo);
        self
    }

    /// Something large in the middle of a card without a photo, such as the
    /// Image Control icon of a recipe.
    pub fn center(mut self, center: impl IntoElement) -> Self {
        self.center = Some(center.into_element());
        self
    }

    /// The Look's cover beside the title.
    pub fn cover(mut self, cover: Cover) -> Self {
        self.mark = HeroMark::Cover(cover);
        self
    }

    /// The mark of nothing in use beside the title.
    pub fn empty_mark(mut self) -> Self {
        self.mark = HeroMark::Empty;
        self
    }

    /// Open the Look or recipe when the card is pressed: a chevron shows.
    pub fn open(mut self, open: impl Fn() + 'static) -> Self {
        self.open = Some(Box::new(open));
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }
}

impl IntoElement for PhotoHero {
    fn into_element(self) -> AnyElement {
        let label = if self.kicker.is_empty() {
            self.title.clone()
        } else {
            format!("{}, {}", self.kicker, self.title)
        };
        let mark: Option<AnyElement> = match self.mark {
            HeroMark::None => None,
            HeroMark::Cover(cover) => Some(
                view()
                    .rounded(px(3.0))
                    .shadow([soft_shadow(2.0, 8.0, 0x0000004D)])
                    .child(cover_thumb(&cover, 56.0, false))
                    .into_element(),
            ),
            HeroMark::Empty => Some(empty_mark()),
        };
        let mut card = view()
            .h(px(HERO_HEIGHT))
            .overflow_hidden()
            .bg(rgb(DARK_FILL))
            .label(label)
            .when_some(self.test_id.as_deref(), |v, id| v.test_id(id))
            .child(self.photo.map(|photo| {
                image(photo)
                    .absolute()
                    .inset_0()
                    .object_fit(ResizeMode::Cover)
            }))
            .child(self.center.map(|center| {
                view()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_center()
                    .child(center)
            }))
            .child(foot_shade(120.0))
            .child(
                view()
                    .absolute()
                    .left(px(14.0))
                    .right(px(14.0))
                    .bottom(px(14.0))
                    .flex_row()
                    .items_end()
                    .gap_3()
                    .child(mark)
                    .child(
                        view()
                            .flex_1()
                            .gap(px(1.0))
                            .child((!self.kicker.is_empty()).then(|| {
                                text(self.kicker)
                                    .text_size(px(crate::ui::ty::CAPTION))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgba(0xFFFFFFC7))
                            }))
                            .child(
                                text(self.title)
                                    .number_of_lines(1)
                                    .text_size(px(crate::ui::ty::TITLE2))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xFFFFFF)),
                            ),
                    )
                    .child(self.open.is_some().then(|| {
                        svg(ImageSrc::svg_markup(
                            r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="14" viewBox="0 0 8 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1.5 1.5l5 5.5-5 5.5"/></svg>"#,
                        ))
                        .w(px(9.0))
                        .h(px(15.0))
                        .mb(px(6.0))
                        .text_color(rgba(0xFFFFFFCC))
                    })),
            );
        if let Some(open) = self.open {
            card = card
                .role(AccessibilityRole::Button)
                .on_press(open)
                .active(|s| s.opacity(0.85));
        }
        card.into_element()
    }
}

/// A shadow under a raised thing on a photo.
fn soft_shadow(y: f32, blur: f32, color: u32) -> BoxShadow {
    BoxShadow {
        offset_x: Points(0.0),
        offset_y: Points(y),
        blur: Points(blur),
        spread: Points(0.0),
        color: rgba(color).into(),
        inset: false,
    }
}

/// A dark shade up from the foot of a photo, so white words on it read.
fn foot_shade(height: f32) -> impl IntoElement {
    view()
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .h(px(height))
        .bg_linear_to_b()
        .gradient_from(rgba(0x00000000))
        .gradient_to(rgba(0x0000009E))
}

/// The mark of nothing in use: a crossed circle on a clear tile the size of
/// a cover.
fn empty_mark() -> AnyElement {
    view()
        .w(px(56.0))
        .h(px(56.0 * 34.0 / 40.0))
        .rounded(px(3.0))
        .border(px(1.5))
        .border_dashed()
        .border_color(rgba(0xFFFFFFB3))
        .items_center()
        .justify_center()
        .child(
            svg(ImageSrc::svg_markup(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="8"/><path d="M6.5 17.5l11-11"/></svg>"#,
            ))
            .size(px(22.0))
            .text_color(rgba(0xFFFFFFD9)),
        )
        .into_element()
}

/// A button in a light capsule over a photo, with a glyph (a 24-unit
/// stroked SVG body) before its title, such as Change Photo.
pub fn overlay_button(
    glyph: &str,
    title: impl Into<String>,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let title = title.into();
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{glyph}</svg>"#
    );
    view()
        .h(px(40.0))
        .pl(px(12.0))
        .pr(px(14.0))
        .rounded_full()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .bg(rgba(0xFFFFFFC7))
        .shadow([soft_shadow(4.0, 14.0, 0x0000002E)])
        .text_color(rgb(0x1C1C1E))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(svg(ImageSrc::svg_markup(&markup)).size(px(18.0)))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x1C1C1E)),
        )
        .into_element()
}

/// The top of a Look or recipe page: the photo with the Look across the
/// page, the page's sheet over its foot with the cover across the sheet's
/// edge, and the title. Build it with [`detail_header`].
pub struct DetailHeader {
    title: String,
    subtitle: Option<String>,
    photo: Option<ImageSrc>,
    cover: Option<Cover>,
    chip: Option<AnyElement>,
    photo_button: Option<AnyElement>,
}

pub fn detail_header(title: impl Into<String>) -> DetailHeader {
    DetailHeader {
        title: title.into(),
        subtitle: None,
        photo: None,
        cover: None,
        chip: None,
        photo_button: None,
    }
}

impl DetailHeader {
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into()).filter(|s: &String| !s.is_empty());
        self
    }

    pub fn photo(mut self, photo: ImageSrc) -> Self {
        self.photo = Some(photo);
        self
    }

    pub fn cover(mut self, cover: Cover) -> Self {
        self.cover = Some(cover);
        self
    }

    /// A state at the end of the title row, such as In Use.
    pub fn chip(mut self, chip: impl IntoElement) -> Self {
        self.chip = Some(chip.into_element());
        self
    }

    /// A button at the photo's lower end, such as [`overlay_button`].
    pub fn photo_button(mut self, button: impl IntoElement) -> Self {
        self.photo_button = Some(button.into_element());
        self
    }
}

impl IntoElement for DetailHeader {
    fn into_element(self) -> AnyElement {
        let has_cover = self.cover.is_some();
        view()
            .child(
                view()
                    .h(px(HEADER_PHOTO_HEIGHT))
                    .overflow_hidden()
                    .bg(rgb(DARK_FILL))
                    .test_id("detail-photo")
                    .child(self.photo.map(|photo| {
                        image(photo)
                            .absolute()
                            .inset_0()
                            .object_fit(ResizeMode::Cover)
                    }))
                    // The status bar and the bar's glass buttons sit on the
                    // photo's top: dark enough there for their white marks
                    // on any photo, such as a bright sky.
                    .child(
                        view()
                            .absolute()
                            .left(px(0.0))
                            .right(px(0.0))
                            .top(px(0.0))
                            .h(px(140.0))
                            .bg_linear_to_b()
                            .gradient_from(rgba(0x00000073))
                            .gradient_to(rgba(0x00000000)),
                    ),
            )
            .child(self.photo_button.map(|button| {
                view()
                    .absolute()
                    .right(px(16.0))
                    .top(px(HEADER_PHOTO_HEIGHT - HEADER_SHEET_OVERLAP - 54.0))
                    .child(button)
            }))
            .child(
                view()
                    .mt(px(-HEADER_SHEET_OVERLAP))
                    .rounded_t(px(HEADER_SHEET_OVERLAP))
                    .bg(move || token("list-background"))
                    .pt(px(if has_cover { 52.0 } else { 24.0 }))
                    .px(px(20.0))
                    .flex_row()
                    .items_end()
                    .justify_between()
                    .gap_3()
                    .child(
                        view()
                            .flex_1()
                            .gap(px(2.0))
                            .child(
                                text(self.title)
                                    .text_size(px(28.0))
                                    .font_weight(FontWeight::BOLD)
                                    .role(AccessibilityRole::Header),
                            )
                            .child(self.subtitle.map(|subtitle| {
                                text(subtitle)
                                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                                    .text_color(move || token("list-secondary"))
                            })),
                    )
                    .child(self.chip),
            )
            .child(self.cover.map(|cover| {
                view()
                    .absolute()
                    .left(px(20.0))
                    .top(px(HEADER_COVER_TOP))
                    .rounded(px(4.0))
                    .shadow([soft_shadow(3.0, 12.0, 0x0000004D)])
                    .test_id("detail-cover")
                    .child(cover_thumb(&cover, HEADER_COVER_SIZE, false))
            }))
            .into_element()
    }
}
