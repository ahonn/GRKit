//! Cover parameters: the `icon` of an effect definition, which `Cover::pixels` draws,
//! and the `cover` of an import library entry. A label tile has `label` and `shape`. A film
//! cover has `template` (`box`, `cine`, or `stack`), `brand`, `name`, an optional `number`,
//! `band` on a box only, and optional `colors`. A GRF holds only the pixels.
//!
//! The site draws the same covers (`web/src/components/EffectIcon.tsx`) and offers the same
//! colors (`web/src/coverPalette.ts`). A test keeps the tables of this module equal to it.

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

pub use crate::artwork::Palette as Colors;
pub use looks::{
    find_look, ink, look_of, packaging_look, random_layout, random_look, Look, EDITOR_LOOKS, INKS,
    LOOKS,
};
use looks::{mix, BRANDS};
use svg::{film_svg, pixel_svg};

mod looks;
mod svg;
#[cfg(test)]
mod tests;

/// An opaque RGB color.
pub type Rgb = [u8; 3];

/// The longest brand, name, or number of a film cover, in characters.
pub const TEXT_MAX: usize = 12;
/// The longest label of a label tile.
pub const LABEL_MAX: usize = 4;
/// The colors of a film cover without `colors`: the classic gray cover.
pub const DEFAULT_COLORS: Colors = crate::artwork::NEUTRAL_PALETTE;

/// The parameters of an effect icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cover {
    Label(Label),
    Film(Film),
}

/// A dark tile with 1 to `LABEL_MAX` capitals or digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub label: String,
    pub shape: Shape,
}

/// The drawing behind the text of a label tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Film perforations above and below the text.
    Film,
    /// A white disc.
    Filter,
    Plain,
}

impl Shape {
    pub const ALL: [Shape; 3] = [Shape::Film, Shape::Filter, Shape::Plain];

    pub fn id(self) -> &'static str {
        match self {
            Shape::Film => "film",
            Shape::Filter => "filter",
            Shape::Plain => "plain",
        }
    }

    pub fn parse(id: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|shape| shape.id() == id)
    }
}

/// A film cover layout, as the site editor offers it. `Box2` is a box with the bottom band:
/// its JSON is `"template": "box", "band": true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// A film box with the brand band at the top.
    Box,
    /// A film box with a light band at the bottom too.
    Box2,
    /// A motion picture film label: the brand band at the top, three stripes at the bottom.
    Cine,
    /// The brand band at the bottom.
    Stack,
}

impl Layout {
    /// The layouts in the order of the site (`COVER_LAYOUTS`).
    pub const ALL: [Layout; 4] = [Layout::Box, Layout::Box2, Layout::Cine, Layout::Stack];

    /// The id of the site editor.
    pub fn id(self) -> &'static str {
        match self {
            Layout::Box => "box",
            Layout::Box2 => "box2",
            Layout::Cine => "cine",
            Layout::Stack => "stack",
        }
    }

    pub fn parse(id: &str) -> Option<Layout> {
        Layout::ALL.into_iter().find(|layout| layout.id() == id)
    }

    /// The `template` of the JSON.
    pub fn template(self) -> &'static str {
        match self {
            Layout::Box | Layout::Box2 => "box",
            Layout::Cine => "cine",
            Layout::Stack => "stack",
        }
    }
}

/// A film cover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Film {
    pub layout: Layout,
    pub brand: String,
    pub name: String,
    /// None shows the name large.
    pub number: Option<String>,
    /// None draws `DEFAULT_COLORS`.
    pub colors: Option<Colors>,
}

impl Film {
    /// The colors that the cover draws.
    pub fn colors(&self) -> Colors {
        self.colors.unwrap_or(DEFAULT_COLORS)
    }

    /// What the random button of the site editor does: another editor look and another
    /// layout, chosen by `seed`.
    pub fn shuffle(&mut self, seed: u64) {
        // By the editor looks: another look can have the same colors.
        let colors = self.colors();
        let current = EDITOR_LOOKS
            .into_iter()
            .find(|id| find_look(id).is_some_and(|look| look.colors() == colors));
        self.colors = Some(random_look(seed, current).colors());
        self.layout = random_layout(mix(seed ^ 0x9e37_79b9_7f4a_7c15), Some(self.layout));
    }
}

/// A cover text: 1 to `TEXT_MAX` printable ASCII characters, as the site and `artwork` check.
pub fn valid_text(text: &str) -> bool {
    (1..=TEXT_MAX).contains(&text.len()) && text.bytes().all(|c| (0x20..=0x7e).contains(&c))
}

/// A tile label: 1 to `LABEL_MAX` capitals or digits.
pub fn valid_label(label: &str) -> bool {
    (1..=LABEL_MAX).contains(&label.len())
        && label
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

const FILM_KEYS: [&str; 6] = ["template", "brand", "name", "number", "band", "colors"];
const COLOR_KEYS: [&str; 5] = ["band", "brand", "body", "text", "accent"];

impl Cover {
    /// Reads cover JSON. It refuses unknown keys and values, and texts that the camera cannot
    /// draw. A text can still be too wide (`fits`).
    pub fn from_json(value: &Value) -> Result<Cover> {
        let object = value.as_object().context("Expected a cover object")?;
        let text = |key: &str| -> Result<String> {
            let text = object
                .get(key)
                .and_then(Value::as_str)
                .with_context(|| format!("Expected cover {key}"))?;
            ensure!(
                valid_text(text),
                "Expected 1 to {TEXT_MAX} printable ASCII characters in the cover {key}"
            );
            Ok(text.to_owned())
        };
        let Some(template) = object.get("template") else {
            ensure!(
                object.len() == 2,
                "Expected a label tile with label and shape only"
            );
            let label = object
                .get("label")
                .and_then(Value::as_str)
                .filter(|label| valid_label(label))
                .context("Expected a label of one to four capitals or digits")?;
            let shape = object
                .get("shape")
                .and_then(Value::as_str)
                .and_then(Shape::parse)
                .context("Expected label shape film, filter, or plain")?;
            return Ok(Cover::Label(Label {
                label: label.to_owned(),
                shape,
            }));
        };
        if let Some(key) = object.keys().find(|key| !FILM_KEYS.contains(&key.as_str())) {
            bail!("Unknown cover key {key}");
        }
        let band = object.get("band").map(|band| {
            band.as_bool()
                .context("Expected a true or false cover band")
        });
        let layout = match (template.as_str(), band.transpose()?) {
            (Some("box"), Some(false)) => Layout::Box,
            (Some("box"), Some(true)) => Layout::Box2,
            (Some("box"), None) => bail!("Expected a box cover band"),
            (Some("cine"), None) => Layout::Cine,
            (Some("stack"), None) => Layout::Stack,
            (Some("cine" | "stack"), Some(_)) => bail!("Only a box cover has a band"),
            _ => bail!("Expected cover template box, cine, or stack"),
        };
        let colors = object
            .get("colors")
            .map(|colors| -> Result<Colors> {
                ensure!(
                    colors.as_object().is_some_and(|colors| colors.len() == 5
                        && COLOR_KEYS.iter().all(|key| colors.contains_key(*key))),
                    "Expected cover colors band, brand, body, text, and accent"
                );
                crate::artwork::parse_colors(colors)
            })
            .transpose()?;
        Ok(Cover::Film(Film {
            layout,
            brand: text("brand")?,
            name: text("name")?,
            number: object
                .contains_key("number")
                .then(|| text("number"))
                .transpose()?,
            colors,
        }))
    }

    /// The JSON of the cover, as `from_json` reads it and the site stores it.
    pub fn to_json(&self) -> Value {
        match self {
            Cover::Label(label) => json!({"label": label.label, "shape": label.shape.id()}),
            Cover::Film(film) => {
                let mut object = Map::new();
                object.insert("template".into(), film.layout.template().into());
                object.insert("brand".into(), film.brand.clone().into());
                object.insert("name".into(), film.name.clone().into());
                if let Some(number) = &film.number {
                    object.insert("number".into(), number.clone().into());
                }
                if matches!(film.layout, Layout::Box | Layout::Box2) {
                    object.insert("band".into(), (film.layout == Layout::Box2).into());
                }
                if let Some(colors) = &film.colors {
                    object.insert("colors".into(), colors_json(colors));
                }
                Value::Object(object)
            }
        }
    }

    /// The 40 x 40 RGBA icon of the cover.
    pub fn pixels(&self) -> Result<Vec<u8>> {
        use crate::artwork::{render_box, render_cine, render_stack, render_tile};
        let film = match self {
            Cover::Label(label) => return render_tile(&label.label, label.shape.id()),
            Cover::Film(film) => film,
        };
        let (brand, name) = (film.brand.as_str(), film.name.as_str());
        let number = film.number.as_deref().unwrap_or("");
        let colors = film.colors.unwrap_or(DEFAULT_COLORS);
        match film.layout {
            Layout::Box => render_box(brand, name, number, false, colors),
            Layout::Box2 => render_box(brand, name, number, true, colors),
            Layout::Cine => render_cine(brand, name, number, colors),
            Layout::Stack => render_stack(brand, name, number, colors),
        }
    }

    /// True when the camera can draw the cover: its texts fit.
    pub fn fits(&self) -> bool {
        self.pixels().is_ok()
    }

    /// True when the cover draws `icon`, the icon of a GRF.
    pub fn matches(&self, icon: &[u8]) -> bool {
        self.pixels().is_ok_and(|pixels| pixels == icon)
    }

    /// The default cover of a Look named `name`: a film box with a band. A known film brand
    /// of the name becomes the brand (else GRKit), and its first film speed, such as 400 or
    /// 200T, the number.
    pub fn for_name(name: &str) -> Cover {
        let cut = |text: &str| -> String {
            text.chars()
                .take(TEXT_MAX)
                .collect::<String>()
                .trim()
                .to_owned()
        };
        let words: Vec<&str> = name.split_whitespace().collect();
        let brand_at = words
            .iter()
            .position(|w| BRANDS.iter().any(|(k, _)| w.eq_ignore_ascii_case(k)));
        let brand = brand_at
            .and_then(|i| {
                BRANDS
                    .iter()
                    .find(|(k, _)| words[i].eq_ignore_ascii_case(k))
            })
            .map_or("GRKit", |(_, brand)| *brand);
        let is_number = |w: &str| {
            let digits = w.bytes().take_while(u8::is_ascii_digit).count();
            digits >= 2
                && w.len() - digits <= 1
                && w.bytes().skip(digits).all(|b| b.is_ascii_alphabetic())
        };
        let number = words.iter().find(|w| is_number(w)).map(|w| cut(w));
        let rest: Vec<&str> = words
            .iter()
            .enumerate()
            .filter(|(i, w)| Some(*i) != brand_at && Some(cut(w)) != number)
            .map(|(_, w)| *w)
            .collect();
        let mut title = cut(&rest.join(" ").to_uppercase());
        if title.is_empty() {
            title = cut(&name.to_uppercase());
        }
        if title.is_empty() {
            title = "LOOK".into();
        }
        Cover::Film(Film {
            layout: Layout::Box2,
            brand: brand.to_owned(),
            name: title,
            number: number.filter(|n| !n.is_empty()),
            colors: None,
        })
    }

    /// A standalone SVG document of the cover, `viewBox="0 0 40 40"`. A film cover draws as
    /// the site draws it, in IBM Plex Sans Condensed Bold. Its texts are outlines, because
    /// the app draws an SVG with the fonts of the device, which do not have this font.
    /// A label tile draws its pixels as a grid of rectangles, as the site shows its pixels.
    pub fn svg(&self) -> String {
        match self {
            Cover::Label(_) => pixel_svg(&self.pixels().unwrap_or_default()),
            Cover::Film(film) => film_svg(film),
        }
    }
}

fn colors_json(colors: &Colors) -> Value {
    json!({"band": colors.band, "brand": colors.brand, "body": colors.body,
        "text": colors.text, "accent": colors.accent})
}

/// The cover of an official Look: its statement cover, or that cover in the colors of its
/// film box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// The cover of the release statement: the classic gray.
    Gray,
    /// The statement cover with the colors of its film box (`Cover::film_box`).
    FilmBox,
}

impl Variant {
    /// This cover of an official Look with the statement cover `statement`.
    pub fn cover(self, statement: &Cover) -> Cover {
        match self {
            Variant::Gray => statement.clone(),
            Variant::FilmBox => statement.film_box(),
        }
    }
}

/// The color scheme of the covers of the official Looks on a card. The scheme is not
/// stored: the covers of the card show it (`Scheme::of`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scheme {
    /// Each official Look has the gray cover.
    #[default]
    Gray,
    /// A color Look has the film box cover, a B&W Look the gray cover.
    ColorFilmBox,
    /// Each official Look has the film box cover.
    FilmBox,
}

impl Scheme {
    const ALL: [Scheme; 3] = [Scheme::Gray, Scheme::ColorFilmBox, Scheme::FilmBox];

    /// The scheme of a card without a cover that shows one, as the site's default. Covers
    /// already on the card decide their own group, so a card of gray color Looks stays gray.
    pub const NEW_CARD: Scheme = Scheme::ColorFilmBox;

    /// The id of the API and the site.
    pub fn id(self) -> &'static str {
        match self {
            Scheme::Gray => "gray",
            Scheme::ColorFilmBox => "colorFilmBox",
            Scheme::FilmBox => "filmBox",
        }
    }

    pub fn parse(id: &str) -> Option<Scheme> {
        Scheme::ALL.into_iter().find(|scheme| scheme.id() == id)
    }

    /// The cover variant of this scheme for an official Look.
    pub fn variant(self, monochrome: bool) -> Variant {
        match (self, monochrome) {
            (Scheme::FilmBox, _) | (Scheme::ColorFilmBox, false) => Variant::FilmBox,
            _ => Variant::Gray,
        }
    }

    /// The cover of this scheme for an official Look with the statement cover `statement`.
    pub fn cover(self, statement: &Cover, monochrome: bool) -> Cover {
        self.variant(monochrome).cover(statement)
    }

    /// The scheme of a card from the variant of each official Look without an individual
    /// cover, and whether the Look is monochrome. Each group, color and B&W, has the film box
    /// cover when each Look of the group has it; a group without Looks does not decide.
    /// `FilmBox` needs both groups with the film box cover, `ColorFilmBox` the color group.
    /// Anything else, such as only B&W Looks with the film box cover, is `Gray`.
    pub fn of(looks: impl IntoIterator<Item = (bool, Variant)>) -> Scheme {
        Scheme::of_or(looks, Scheme::Gray)
    }

    /// `of`, where a group without Looks has the variant that `fallback` gives it, as the
    /// site falls back to the scheme of the browser (`schemeOnCard`).
    pub fn of_or(looks: impl IntoIterator<Item = (bool, Variant)>, fallback: Scheme) -> Scheme {
        // For each group: whether each Look has the film box, or None without a Look.
        let mut groups = [None; 2];
        for (monochrome, variant) in looks {
            let group = &mut groups[usize::from(monochrome)];
            *group = Some(group.unwrap_or(true) && variant == Variant::FilmBox);
        }
        let boxed = |monochrome: bool| {
            groups[usize::from(monochrome)]
                .unwrap_or(fallback.variant(monochrome) == Variant::FilmBox)
        };
        match (boxed(false), boxed(true)) {
            (true, true) => Scheme::FilmBox,
            (true, false) => Scheme::ColorFilmBox,
            _ => Scheme::Gray,
        }
    }
}

impl Cover {
    /// The cover with the colors of its film box, as the site gives an official Look
    /// (`resolveColors`): its own colors, else the packaging colors of its film. A label tile
    /// stays as it is.
    pub fn film_box(&self) -> Cover {
        let Cover::Film(film) = self else {
            return self.clone();
        };
        let colors = film.colors.unwrap_or_else(|| {
            packaging_look(
                &film.brand,
                &film.name,
                film.layout.template(),
                film.number.as_deref().unwrap_or(""),
            )
            .colors()
        });
        Cover::Film(Film {
            colors: Some(colors),
            ..film.clone()
        })
    }

    /// The variant that `cover` is, when `self` is the statement cover of an official Look.
    /// A cover whose film box is gray is the gray cover.
    pub fn variant_of(&self, cover: &Cover) -> Option<Variant> {
        let drawn = |cover: &Cover| match cover {
            Cover::Film(film) => Cover::Film(Film {
                colors: Some(film.colors()),
                ..film.clone()
            }),
            label => label.clone(),
        };
        let cover = drawn(cover);
        if cover == drawn(self) {
            Some(Variant::Gray)
        } else if cover == drawn(&self.film_box()) {
            Some(Variant::FilmBox)
        } else {
            None
        }
    }

    /// The variant whose cover draws the icon nearest to `icon`, by the number of bytes that
    /// differ, when `self` is the statement cover of an official Look. Another build of the
    /// drawing can change some pixels of a cover, so an icon of a card counts as the film box
    /// cover when it is nearer to it than to the gray cover. A tie, or a film box that draws
    /// the gray icon, is gray.
    pub fn nearest_variant(&self, icon: &[u8]) -> Variant {
        let distance = |cover: &Cover| {
            cover.pixels().map_or(usize::MAX, |pixels| {
                pixels.iter().zip(icon).filter(|(a, b)| a != b).count()
                    + pixels.len().abs_diff(icon.len())
            })
        };
        match distance(&self.film_box()) < distance(self) {
            true => Variant::FilmBox,
            false => Variant::Gray,
        }
    }

    /// The variant whose cover draws `icon`, when `self` is the statement cover of an
    /// official Look.
    pub fn variant_of_icon(&self, icon: &[u8]) -> Option<Variant> {
        if self.matches(icon) {
            Some(Variant::Gray)
        } else if self.film_box().matches(icon) {
            Some(Variant::FilmBox)
        } else {
            None
        }
    }
}

/// The cover of the native look `id` (`gr4_looks::LOOKS`, a color look): the gray cover
/// of its file, as `Cover::for_name` draws without colors. Its film box cover has the colors
/// that the site gives it (`NATIVE_LOOKS`). A native look is a color look whose cover follows
/// the scheme of the official Looks.
pub fn native_cover(id: &str) -> Option<Cover> {
    let look = crate::gr4_looks::LOOKS
        .iter()
        .find(|look| matches!(look.kind, crate::gr4_looks::Kind::Color) && look.effect_id == id)?;
    let (template, name, number) = look.cover;
    Some(Cover::Film(Film {
        layout: match template {
            "cine" => Layout::Cine,
            _ => Layout::Box,
        },
        brand: "RICOH".into(),
        name: name.into(),
        number: Some(number.into()),
        colors: None,
    }))
}
