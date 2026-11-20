//! Cover parameters: the `icon` of an effect definition, which `effects::icon_pixels` draws,
//! and the `cover` of an import library entry. A label tile has `label` and `shape`. A film
//! cover has `template` (`box`, `cine`, or `stack`), `brand`, `name`, an optional `number`,
//! `band` on a box only, and optional `colors`. A GRF holds only the pixels.
//!
//! The site draws the same covers (`web/src/components/EffectIcon.tsx`) and offers the same
//! colors (`web/src/coverPalette.ts`). A test keeps the tables of this module equal to it.

use std::fmt::Write;

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

pub use crate::artwork::Palette as Colors;

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
        crate::effects::icon_pixels(&self.to_json())
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
    /// the site draws it, in IBM Plex Sans Condensed Bold. The site fits a text to its width
    /// with the browser's measure; this document fits it with the advance widths of the font.
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

/// Film brands as the catalog covers write them.
const BRANDS: [(&str, &str); 9] = [
    ("kodak", "Kodak"),
    ("fuji", "FUJIFILM"),
    ("fujifilm", "FUJIFILM"),
    ("ilford", "ILFORD"),
    ("agfa", "AGFA"),
    ("polaroid", "Polaroid"),
    ("rollei", "Rollei"),
    ("cinestill", "CineStill"),
    ("lomography", "Lomography"),
];

/// The inks of the film covers (`COVER_INKS`), in the order of the site.
pub const INKS: [(&str, Rgb); 50] = [
    ("white", [255, 255, 255]),
    ("warmWhite", [255, 236, 220]),
    ("portraCream", [232, 212, 176]),
    ("goldText", [255, 220, 120]),
    ("silver", [236, 236, 236]),
    ("gray", [214, 214, 214]),
    ("steel", [168, 168, 176]),
    ("mist", [200, 200, 200]),
    ("ash", [64, 64, 64]),
    ("charcoal", [48, 48, 48]),
    ("classicBrand", [40, 40, 40]),
    ("classicBody", [51, 51, 51]),
    ("graphite", [36, 36, 36]),
    ("black", [20, 20, 20]),
    ("ink", [16, 16, 16]),
    ("kodakYellow", [242, 196, 24]),
    ("kodakRed", [196, 24, 24]),
    ("ektarRed", [220, 48, 32]),
    ("ricohRed", [220, 32, 40]),
    ("orange", [240, 112, 48]),
    ("agfaOrange", [224, 96, 32]),
    ("vistaOrange", [232, 120, 40]),
    ("amber", [242, 168, 64]),
    ("portraBrown", [96, 56, 40]),
    ("portraAccent", [212, 168, 120]),
    ("brown", [40, 24, 16]),
    ("purple", [96, 40, 128]),
    ("violet", [120, 72, 168]),
    ("chromeBlue", [32, 140, 196]),
    ("teal", [24, 96, 120]),
    ("blueAccent", [64, 144, 200]),
    ("velviaGreen", [8, 88, 64]),
    ("fujiGreen", [28, 120, 64]),
    ("superiaGreen", [36, 160, 64]),
    ("chartreuse", [180, 200, 48]),
    ("forest", [20, 96, 56]),
    ("pine", [16, 88, 72]),
    ("eternaGreen", [16, 56, 40]),
    ("sage", [120, 168, 136]),
    ("sageDark", [40, 48, 40]),
    ("mint", [160, 200, 184]),
    ("moss", [200, 240, 208]),
    ("cineGreen", [56, 160, 88]),
    ("cineGreenBody", [14, 40, 24]),
    ("cineGreenText", [168, 232, 184]),
    ("cineYellowBody", [40, 28, 8]),
    ("greenDark", [28, 64, 28]),
    ("eternaAccent", [120, 140, 120]),
    ("pink", [255, 120, 160]),
    ("rose", [216, 152, 160]),
];

/// The RGB of the ink `id`.
pub fn ink(id: &str) -> Option<Rgb> {
    INKS.iter()
        .find(|(name, _)| *name == id)
        .map(|(_, rgb)| *rgb)
}

/// A named set of cover colors (`COVER_LOOKS`): the ink of the band, brand, body, text, and
/// accent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub id: &'static str,
    pub inks: [&'static str; 5],
}

impl Look {
    pub fn colors(&self) -> Colors {
        let [band, brand, body, text, accent] = self
            .inks
            .map(|id| ink(id).expect("a look names known inks"));
        Colors {
            band,
            brand,
            body,
            text,
            accent,
        }
    }
}

const fn look(id: &'static str, inks: [&'static str; 5]) -> Look {
    Look { id, inks }
}

/// Each look, in the order of the site.
pub const LOOKS: [Look; 35] = [
    // Equal to `DEFAULT_COLORS`.
    look(
        "gray",
        ["gray", "classicBrand", "classicBody", "white", "gray"],
    ),
    look(
        "kodak",
        ["kodakYellow", "kodakRed", "black", "white", "kodakYellow"],
    ),
    look(
        "gold",
        ["kodakYellow", "kodakRed", "black", "white", "kodakRed"],
    ),
    look(
        "ultramax",
        ["purple", "white", "black", "white", "kodakYellow"],
    ),
    look("ektar", ["orange", "white", "black", "white", "ektarRed"]),
    look(
        "ektachrome",
        ["chromeBlue", "white", "black", "white", "kodakYellow"],
    ),
    look(
        "kodachrome",
        ["kodakRed", "white", "black", "white", "kodakYellow"],
    ),
    look(
        "aerocolor",
        ["charcoal", "kodakYellow", "black", "white", "kodakRed"],
    ),
    look(
        "portra",
        [
            "portraCream",
            "portraBrown",
            "black",
            "white",
            "portraAccent",
        ],
    ),
    look(
        "trix",
        ["kodakYellow", "black", "black", "white", "kodakRed"],
    ),
    look("tmax", ["steel", "ink", "black", "white", "kodakYellow"]),
    look(
        "doublex",
        ["charcoal", "kodakYellow", "black", "white", "mist"],
    ),
    look(
        "vision",
        ["charcoal", "kodakYellow", "black", "white", "kodakYellow"],
    ),
    look(
        "velvia",
        ["velviaGreen", "white", "black", "white", "ektarRed"],
    ),
    look("provia", ["teal", "white", "black", "white", "blueAccent"]),
    look("astia", ["sage", "sageDark", "black", "white", "rose"]),
    look(
        "superia",
        ["superiaGreen", "white", "black", "white", "kodakYellow"],
    ),
    look(
        "c200",
        ["chartreuse", "greenDark", "black", "white", "fujiGreen"],
    ),
    look("pro", ["forest", "white", "black", "white", "mint"]),
    look("nps", ["pine", "white", "black", "white", "mint"]),
    look("acros", ["ash", "white", "black", "white", "fujiGreen"]),
    look(
        "instax",
        ["fujiGreen", "white", "white", "graphite", "pink"],
    ),
    look(
        "eterna",
        ["eternaGreen", "moss", "black", "white", "eternaAccent"],
    ),
    look("hp5", ["ash", "white", "black", "white", "kodakYellow"]),
    look("delta", ["silver", "ink", "black", "white", "kodakYellow"]),
    look("xp2", ["violet", "white", "black", "white", "kodakYellow"]),
    look(
        "precisa",
        ["agfaOrange", "white", "black", "white", "ektarRed"],
    ),
    look("vista", ["vistaOrange", "brown", "black", "white", "amber"]),
    look("cinema", ["ash", "mist", "ink", "white", "steel"]),
    look(
        "cinemaYellow",
        [
            "ricohRed",
            "white",
            "cineYellowBody",
            "goldText",
            "kodakYellow",
        ],
    ),
    look(
        "cinemaGreen",
        [
            "ricohRed",
            "white",
            "cineGreenBody",
            "cineGreenText",
            "cineGreen",
        ],
    ),
    look("ricoh", ["ricohRed", "white", "ink", "white", "mist"]),
    look(
        "fuji",
        ["fujiGreen", "white", "black", "white", "fujiGreen"],
    ),
    look("ilford", ["ink", "white", "black", "white", "kodakYellow"]),
    look(
        "agfa",
        ["agfaOrange", "white", "black", "white", "ektarRed"],
    ),
];

/// The looks that the site editor offers (`COVER_LOOK_IDS`), in its order.
pub const EDITOR_LOOKS: [&str; 35] = [
    "gray",
    "kodak",
    "gold",
    "ultramax",
    "portra",
    "ektar",
    "ektachrome",
    "kodachrome",
    "aerocolor",
    "vision",
    "trix",
    "tmax",
    "doublex",
    "fuji",
    "superia",
    "c200",
    "pro",
    "nps",
    "velvia",
    "provia",
    "astia",
    "instax",
    "eterna",
    "acros",
    "ilford",
    "hp5",
    "delta",
    "xp2",
    "agfa",
    "precisa",
    "vista",
    "ricoh",
    "cinemaYellow",
    "cinemaGreen",
    "cinema",
];

/// The look `id`.
pub fn find_look(id: &str) -> Option<&'static Look> {
    LOOKS.iter().find(|look| look.id == id)
}

/// The first look with `colors`, as the site editor finds the look of a cover.
pub fn look_of(colors: &Colors) -> Option<&'static Look> {
    LOOKS.iter().find(|look| look.colors() == *colors)
}

/// The look of each film stock (`STOCK_LOOKS`): a JavaScript regular expression for the
/// capitals of "NAME NUMBER BRAND", in the order of the site. `regex_matches` reads it.
const STOCK_LOOKS: [(&str, &str); 29] = [
    ("PORTRA", "portra"),
    ("GOLD", "gold"),
    ("ULTRAMAX", "ultramax"),
    ("EKTAR", "ektar"),
    ("EKTACHROME", "ektachrome"),
    ("KODACHROME", "kodachrome"),
    ("AEROCOLOR", "aerocolor"),
    ("TRI-?X", "trix"),
    ("T-?MAX", "tmax"),
    ("DOUBLE-?X|5222", "doublex"),
    ("VISION|5207|5219", "vision"),
    ("VELVIA", "velvia"),
    ("PROVIA", "provia"),
    ("ASTIA", "astia"),
    ("SUPERIA", "superia"),
    ("FUJICOLOR|C200", "c200"),
    ("160C", "nps"),
    (r"\bPRO\b|400H", "pro"),
    ("ACROS|NEOPAN", "acros"),
    ("INSTAX", "instax"),
    ("ETERNA|8573", "eterna"),
    ("HP5", "hp5"),
    ("DELTA", "delta"),
    ("XP2", "xp2"),
    ("PRECISA", "precisa"),
    ("VISTA", "vista"),
    ("CINEMA YELLOW|YELLOW RICOH", "cinemaYellow"),
    ("CINEMA GREEN|GREEN RICOH", "cinemaGreen"),
    ("CINEMA|RICOH", "ricoh"),
];

/// The look of each film brand (`BRAND_LOOKS`): a regular expression for the brand that
/// ignores case, in the order of the site.
const BRAND_LOOKS: [(&str, &str); 4] = [
    ("kodak|eastman", "kodak"),
    ("fuji", "fuji"),
    ("ilford|harman", "ilford"),
    ("agfa", "agfa"),
];

/// True when the JavaScript regular expression `pattern` matches in `text`. It reads only
/// the syntax of the packaging tables: characters, `?` after a character, `\b`, and `|`.
fn regex_matches(pattern: &str, text: &str, ignore_case: bool) -> bool {
    #[derive(Clone, Copy)]
    enum Token {
        Char(u8),
        Optional(u8),
        Boundary,
    }
    fn at(tokens: &[Token], text: &[u8], i: usize, ignore_case: bool) -> bool {
        let Some((first, rest)) = tokens.split_first() else {
            return true;
        };
        let is = |c: u8| {
            text.get(i).is_some_and(|t| match ignore_case {
                true => t.eq_ignore_ascii_case(&c),
                false => *t == c,
            })
        };
        let word = |i: Option<usize>| {
            i.and_then(|i| text.get(i))
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        };
        match *first {
            Token::Char(c) => is(c) && at(rest, text, i + 1, ignore_case),
            Token::Optional(c) => {
                (is(c) && at(rest, text, i + 1, ignore_case)) || at(rest, text, i, ignore_case)
            }
            Token::Boundary => {
                word(i.checked_sub(1)) != word(Some(i)) && at(rest, text, i, ignore_case)
            }
        }
    }
    pattern.split('|').any(|alternative| {
        let mut tokens = Vec::new();
        let mut bytes = alternative.bytes().peekable();
        while let Some(c) = bytes.next() {
            tokens.push(match (c, bytes.peek()) {
                (b'\\', Some(b'b')) => {
                    bytes.next();
                    Token::Boundary
                }
                (c, Some(b'?')) => {
                    bytes.next();
                    Token::Optional(c)
                }
                (c, _) => Token::Char(c),
            });
        }
        (0..=text.len()).any(|i| at(&tokens, text.as_bytes(), i, ignore_case))
    })
}

/// The look of the film box of a cover (`packagingLook`): the look of its film stock, else
/// `cinema` for a `cine` cover, else the look of its brand, else gray.
pub fn packaging_look(brand: &str, name: &str, template: &str, number: &str) -> &'static Look {
    let label = format!("{name} {number} {brand}").to_ascii_uppercase();
    let id = STOCK_LOOKS
        .iter()
        .find(|(pattern, _)| regex_matches(pattern, &label, false))
        .map(|(_, id)| *id)
        .or((template == "cine").then_some("cinema"))
        .or_else(|| {
            BRAND_LOOKS
                .iter()
                .find(|(pattern, _)| regex_matches(pattern, brand, true))
                .map(|(_, id)| *id)
        })
        .unwrap_or("gray");
    find_look(id).expect("each packaging look is a look")
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

/// The cover of the native look `id` (`gr4_looks::LOOKS`, the offered looks): the gray cover
/// of its file, as `Cover::for_name` draws without colors. Its film box cover has the colors
/// that the site gives it (`NATIVE_LOOKS`). A native look is a color look whose cover follows
/// the scheme of the official Looks.
pub fn native_cover(id: &str) -> Option<Cover> {
    let look = crate::gr4_looks::LOOKS
        .iter()
        .find(|look| look.offered && look.effect_id == id)?;
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

/// An editor look chosen by `seed`, other than `except` when there is a choice.
pub fn random_look(seed: u64, except: Option<&str>) -> &'static Look {
    let id = pick(&EDITOR_LOOKS, seed, except);
    find_look(id).expect("each editor look is a look")
}

/// A layout chosen by `seed`, other than `except` when there is a choice.
pub fn random_layout(seed: u64, except: Option<Layout>) -> Layout {
    pick(&Layout::ALL, seed, except)
}

fn pick<T: Copy + PartialEq>(items: &[T], seed: u64, except: Option<T>) -> T {
    let others: Vec<T> = items
        .iter()
        .copied()
        .filter(|item| Some(*item) != except)
        .collect();
    let pool = if others.is_empty() { items } else { &others };
    pool[(mix(seed) % pool.len() as u64) as usize]
}

/// SplitMix64: spreads close seeds, such as a counter or a time, over the choices.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The advance widths of IBM Plex Sans Condensed Bold (Latin subset, version 5.3), in
/// thousandths of the font size, for the characters ' ' to '~'.
const ADVANCE: [u16; 95] = [
    212, 292, 452, 561, 535, 884, 652, 244, 304, 304, 556, 540, 284, 360, 284, 422, 540, 540, 540,
    540, 540, 540, 540, 540, 540, 540, 304, 304, 540, 540, 540, 452, 814, 618, 602, 589, 630, 549,
    528, 646, 654, 397, 506, 630, 480, 738, 654, 642, 595, 642, 610, 556, 527, 630, 598, 884, 612,
    579, 546, 302, 422, 302, 540, 500, 540, 516, 550, 468, 550, 508, 341, 499, 540, 262, 266, 524,
    277, 808, 540, 508, 550, 550, 367, 455, 342, 540, 490, 756, 501, 496, 466, 338, 370, 338, 540,
];

/// The size, letter spacing, and width of a cover text (`styles.css`, `EffectIcon.tsx`).
struct Style {
    size: f32,
    spacing: f32,
    width: f32,
}

const BRAND: Style = Style {
    size: 8.0,
    spacing: 0.0,
    width: 34.0,
};
const NAME: Style = Style {
    size: 6.6,
    spacing: 0.2,
    width: 38.0,
};
const NUMBER: Style = Style {
    size: 14.0,
    spacing: -0.3,
    width: 36.0,
};
const LARGE_NAME: Style = Style {
    size: 12.0,
    spacing: -0.2,
    width: 38.0,
};
/// The smallest size of a large name, and the capital height as a share of the size.
const MIN_LARGE_SIZE: f32 = 5.5;
const CAP_HEIGHT: f32 = 0.7;

/// The width of `text` as a browser measures it: the advances and the spacing after each
/// character.
fn text_width(text: &str, size: f32, spacing: f32) -> f32 {
    let advances: u32 = text
        .bytes()
        .map(|c| u32::from(ADVANCE[usize::from(c.clamp(b' ', b'~') - b' ')]))
        .sum();
    advances as f32 * size / 1000.0 + spacing * text.len() as f32
}

/// A number without trailing zeros.
fn number(value: f32) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c => escaped.push(c),
        }
    }
    escaped
}

fn rect(svg: &mut String, y: f32, height: f32, fill: Rgb) {
    let _ = write!(
        svg,
        r#"<rect x="0" y="{}" width="40" height="{}" fill="{}"/>"#,
        number(y),
        number(height),
        hex(fill)
    );
}

/// A text centered on x = 20 at `size`. A text wider than the style width compresses to it,
/// as `fit` of the site does.
fn text(svg: &mut String, content: &str, y: f32, style: &Style, size: f32, fill: Rgb) {
    let _ = write!(
        svg,
        r#"<text x="20" y="{}" font-size="{}" fill="{}""#,
        number(y),
        number(size),
        hex(fill)
    );
    if style.spacing != 0.0 {
        let _ = write!(svg, r#" letter-spacing="{}""#, number(style.spacing));
    }
    if text_width(content, size, style.spacing) > style.width {
        let _ = write!(
            svg,
            r#" textLength="{}" lengthAdjust="spacingAndGlyphs""#,
            number(style.width)
        );
    }
    let _ = write!(svg, ">{}</text>", escape(content));
}

/// A large name: smaller until it fits, down to `MIN_LARGE_SIZE`, with its capitals centered
/// on `center`, as `fitLarge` of the site does.
fn large_name(svg: &mut String, name: &str, center: f32, fill: Rgb) {
    let style = &LARGE_NAME;
    let length = text_width(name, style.size, style.spacing);
    let size = if length > style.width {
        (style.size * style.width / length).max(MIN_LARGE_SIZE)
    } else {
        style.size
    };
    text(
        svg,
        name,
        center + size * CAP_HEIGHT / 2.0,
        style,
        size,
        fill,
    );
}

fn film_svg(film: &Film) -> String {
    let colors = film.colors();
    let mut svg = String::from(concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40""#,
        r#" font-family="'IBM Plex Sans Condensed', sans-serif" font-weight="700""#,
        r#" text-anchor="middle">"#
    ));
    rect(&mut svg, 3.0, 34.0, colors.body);
    // The rows of the brand, the name, and the number, and the center of a large name.
    let (brand_y, name_y, number_y, center) = if film.layout == Layout::Stack {
        let footer = 28.0;
        rect(&mut svg, footer, 37.0 - footer, colors.band);
        (34.8, 13.0, 24.5, (3.0 + footer) / 2.0)
    } else {
        let header_end = 12.0;
        rect(&mut svg, 3.0, header_end - 3.0, colors.band);
        let (number_y, body_end) = match film.layout {
            Layout::Box2 => {
                rect(&mut svg, 34.0, 3.0, colors.accent);
                (32.0, 34.0)
            }
            Layout::Cine => {
                for y in [32.0, 34.0, 36.0] {
                    rect(&mut svg, y, 1.0, colors.accent);
                }
                (30.2, 32.0)
            }
            _ => (33.0, 37.0),
        };
        (10.4, 19.0, number_y, (header_end + body_end) / 2.0)
    };
    text(
        &mut svg,
        &film.brand,
        brand_y,
        &BRAND,
        BRAND.size,
        colors.brand,
    );
    match &film.number {
        Some(value) => {
            text(&mut svg, &film.name, name_y, &NAME, NAME.size, colors.text);
            text(&mut svg, value, number_y, &NUMBER, NUMBER.size, colors.text);
        }
        None => large_name(&mut svg, &film.name, center, colors.text),
    }
    svg.push_str("</svg>");
    svg
}

/// 40 x 40 RGBA pixels as rectangles: one for each run of equal opaque pixels in a row.
fn pixel_svg(rgba: &[u8]) -> String {
    let mut svg = String::from(concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40""#,
        r#" shape-rendering="crispEdges">"#
    ));
    let pixel = |x: usize, y: usize| {
        let at = (y * 40 + x) * 4;
        rgba.get(at..at + 4)
            .filter(|p| p[3] != 0)
            .map(|p| [p[0], p[1], p[2]])
    };
    for y in 0..40 {
        let mut x = 0;
        while x < 40 {
            let Some(color) = pixel(x, y) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < 40 && pixel(x, y) == Some(color) {
                x += 1;
            }
            let _ = write!(
                svg,
                r#"<rect x="{start}" y="{y}" width="{}" height="1" fill="{}"/>"#,
                x - start,
                hex(color)
            );
        }
    }
    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xmp::xml::{parse_document, Node};

    #[test]
    fn json_round_trips_and_rejects_what_the_camera_cannot_draw() {
        for value in [
            json!({"label": "FV", "shape": "film"}),
            json!({"template": "box", "brand": "Kodak", "name": "PORTRA", "number": "400", "band": true}),
            json!({"template": "box", "brand": "ILFORD", "name": "XP2 SUPER", "band": false,
                "colors": {"band": [1, 2, 3], "brand": [4, 5, 6], "body": [7, 8, 9],
                    "text": [10, 11, 12], "accent": [13, 14, 255]}}),
            json!({"template": "cine", "brand": "Kodak", "name": "5207", "number": "250D"}),
            json!({"template": "stack", "brand": "AGFA", "name": "VISTA"}),
        ] {
            let cover = Cover::from_json(&value).unwrap();
            assert_eq!(cover.to_json(), value);
            assert!(cover.fits(), "{value}");
        }
        let film = json!({"template": "box", "brand": "Kodak", "name": "GOLD", "band": false});
        for (key, bad) in [
            ("template", json!("box2")),
            ("band", json!("yes")),
            ("name", json!("")),
            ("name", json!("THIRTEEN CHAR")),
            ("name", json!("FUJI\u{e9}")),
            ("number", json!("")),
            ("extra", json!(1)),
            ("colors", json!({"band": [0, 0, 0]})),
        ] {
            let mut value = film.clone();
            value[key] = bad;
            assert!(Cover::from_json(&value).is_err(), "{value}");
        }
        for bad in [
            json!({"template": "box", "brand": "Kodak", "name": "GOLD"}),
            json!({"template": "cine", "brand": "Kodak", "name": "GOLD", "band": true}),
            json!({"label": "fv", "shape": "film"}),
            json!({"label": "FV", "shape": "round"}),
            json!({"label": "FV", "shape": "film", "band": true}),
            json!("box"),
        ] {
            assert!(Cover::from_json(&bad).is_err(), "{bad}");
        }
        // Valid text that is too wide for the camera.
        let wide = json!({"template": "box", "brand": "Kodak", "name": "WWWWWWWWWWWW",
            "number": "WWWWWWWWWWWW", "band": false});
        assert!(!Cover::from_json(&wide).unwrap().fits());
    }

    #[test]
    fn a_cover_matches_only_its_own_pixels() {
        let cover = Cover::for_name("Kodak Portra 400");
        let pixels = cover.pixels().unwrap();
        assert!(cover.matches(&pixels));
        let Cover::Film(mut film) = cover else {
            unreachable!()
        };
        film.colors = Some(find_look("kodak").unwrap().colors());
        assert!(!Cover::Film(film).matches(&pixels));
        assert!(!Cover::for_name("Kodak Portra 400").matches(&pixels[1..]));
    }

    /// The text of each `<text>` of an SVG document, which must be well-formed XML.
    fn svg_texts(svg: &str) -> Vec<String> {
        let root = parse_document(svg).unwrap();
        assert_eq!(
            (root.name.as_str(), root.attr("viewBox")),
            ("svg", Some("0 0 40 40"))
        );
        root.children
            .iter()
            .filter_map(|node| match node {
                Node::Element(element) if element.name == "text" => Some(element.text_content()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn svg_is_well_formed_with_escaped_texts() {
        let brand = r#"A&B <"'>"#;
        for (layout, number) in [
            (Layout::Box, Some("400")),
            (Layout::Box2, None),
            (Layout::Cine, Some("5<7")),
            (Layout::Stack, None),
        ] {
            let film = Film {
                layout,
                brand: brand.into(),
                name: "LONGER NAMES".into(),
                number: number.map(str::to_owned),
                colors: None,
            };
            let svg = Cover::Film(film).svg();
            let mut expected = vec![brand, "LONGER NAMES"];
            expected.extend(number);
            assert_eq!(svg_texts(&svg), expected, "{layout:?}");
            assert!(svg.contains(&hex(DEFAULT_COLORS.body)));
            // A large name this wide gets smaller.
            assert!(!svg.contains(r#"font-size="12""#));
        }
        let label = Cover::from_json(&json!({"label": "FV", "shape": "filter"})).unwrap();
        let svg = label.svg();
        assert!(svg_texts(&svg).is_empty());
        assert!(svg.matches("<rect").count() > 40);
    }

    /// The entries of the TypeScript table that starts at `start`, up to `end`: each
    /// `key: value` at the start of a line or after `{`, in order.
    fn table<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let from = source.find(start).expect("the table start") + start.len();
        let to = from + source[from..].find(end).expect("the table end");
        &source[from..to]
    }

    #[test]
    fn palette_equals_the_site() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/coverPalette.ts");
        let source = std::fs::read_to_string(path).unwrap();
        let code = |text: &str| -> String {
            text.lines()
                .filter(|line| !line.trim_start().starts_with("//") && !line.contains("/**"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let inks: Vec<(String, Rgb)> = code(table(
            &source,
            "export const COVER_INKS = {",
            "} as const satisfies",
        ))
        .lines()
        .filter_map(|line| {
            let (id, rgb) = line.trim().split_once(": [")?;
            let rgb: Vec<u8> = rgb
                .trim_end_matches("],")
                .split(", ")
                .map(|v| v.parse().unwrap())
                .collect();
            Some((id.to_owned(), rgb.try_into().unwrap()))
        })
        .collect();
        let ours: Vec<(String, Rgb)> = INKS.iter().map(|(id, rgb)| ((*id).into(), *rgb)).collect();
        assert_eq!(ours, inks);

        let looks = code(table(
            &source,
            "export const COVER_LOOKS = {",
            "} as const satisfies",
        ));
        let looks: Vec<(String, Vec<(String, String)>)> = looks
            .split('}')
            .filter_map(|entry| {
                let (id, slots) = entry.split_once('{')?;
                let id = id
                    .trim()
                    .trim_start_matches(',')
                    .trim()
                    .trim_end_matches(':');
                let slots = slots
                    .split(',')
                    .filter_map(|slot| {
                        let (key, ink) = slot.split_once(':')?;
                        Some((
                            key.trim().to_owned(),
                            ink.trim().trim_matches('\'').to_owned(),
                        ))
                    })
                    .collect();
                Some((id.to_owned(), slots))
            })
            .collect();
        assert_eq!(looks.len(), LOOKS.len());
        for (ours, (id, slots)) in LOOKS.iter().zip(&looks) {
            assert_eq!(ours.id, id);
            for (key, ink) in COLOR_KEYS.iter().zip(ours.inks) {
                assert!(slots.contains(&((*key).into(), ink.into())), "{id}.{key}");
            }
            assert_eq!(slots.len(), 5, "{id}");
        }

        let editor: Vec<&str> = table(&source, "export const COVER_LOOK_IDS = [", "] as const")
            .split(',')
            .map(|id| id.trim().trim_matches('\''))
            .filter(|id| !id.is_empty())
            .collect();
        assert_eq!(editor, EDITOR_LOOKS);
        let layouts = Layout::ALL
            .map(|layout| format!("'{}'", layout.id()))
            .join(", ");
        assert!(source.contains(&format!("COVER_LAYOUTS = [{layouts}] as const")));
        assert_eq!(find_look("gray").unwrap().colors(), DEFAULT_COLORS);
    }

    /// The packaging rule equals `packagingLook` of the site. The tables have the same regular
    /// expression sources, flags, and looks in the same order, and use only the syntax that
    /// `regex_matches` reads. The expected looks below are the output of the TypeScript
    /// `packagingLook` (run with `node --experimental-strip-types`) for the statement cover of
    /// each official Look in `effects/*/effect.json` and for cases of the other rules.
    #[test]
    fn film_box_equals_the_site() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/coverPalette.ts");
        let source = std::fs::read_to_string(path).unwrap();
        let rules = |start: &str| -> Vec<(String, String, String)> {
            table(&source, start, "];")
                .lines()
                .filter_map(|line| {
                    let rest = line.trim().strip_prefix("{ match: /")?;
                    let (regex, rest) = rest.split_once(", look: '")?;
                    let (pattern, flags) = regex.rsplit_once('/')?;
                    let look = rest.split_once('\'')?.0;
                    Some((pattern.to_owned(), flags.to_owned(), look.to_owned()))
                })
                .collect()
        };
        let ours = |table: &[(&str, &str)], flags: &str| -> Vec<(String, String, String)> {
            table
                .iter()
                .map(|(pattern, look)| ((*pattern).into(), flags.into(), (*look).into()))
                .collect()
        };
        assert_eq!(rules("const STOCK_LOOKS"), ours(&STOCK_LOOKS, ""));
        assert_eq!(rules("const BRAND_LOOKS"), ours(&BRAND_LOOKS, "i"));
        let syntax = |pattern: &str| {
            pattern
                .replace(r"\b", "")
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b" -|".contains(&c) || c == b'?')
        };
        for (pattern, _) in STOCK_LOOKS.iter().chain(&BRAND_LOOKS) {
            assert!(syntax(pattern), "{pattern}");
        }
        assert!(STOCK_LOOKS
            .iter()
            .chain(&BRAND_LOOKS)
            .all(|(_, look)| find_look(look).is_some()));
        assert!(source.contains(
            "const label = `${name} ${number} ${brand}`.toUpperCase();\n  for (const stock of STOCK_LOOKS) {\n    if (stock.match.test(label)) return stock.look;\n  }\n  if (template === 'cine') return 'cinema';\n  for (const entry of BRAND_LOOKS) {\n    if (entry.match.test(brand)) return entry.look;\n  }\n  return 'gray';"
        ));

        let official = [
            ("aerocolor-iv-v1", "aerocolor"),
            ("astia-100f-v1", "astia"),
            ("c200-premier-v1", "c200"),
            ("delta-100-v1", "delta"),
            ("double-x-5222-mono-v1", "doublex"),
            ("ektachrome-e100-v1", "ektachrome"),
            ("ektar-100-edge-v1", "ektar"),
            ("eterna-500t-3523xd-v1", "eterna"),
            ("fuji-160c-v1", "nps"),
            ("gold-200-premier-v1", "gold"),
            ("hp5-plus-400-mono-v1", "hp5"),
            ("instax-v1", "instax"),
            ("kodachrome-64-v1", "kodachrome"),
            ("neopan-acros-100-v1", "acros"),
            ("portra-160-vc-v1", "portra"),
            ("portra-400-endura-v1", "portra"),
            ("portra-800-push2-v1", "portra"),
            ("precisa-100-v1", "precisa"),
            ("pro-400h-crystal-v1", "pro"),
            ("provia-100f-v1", "provia"),
            ("red-filter-mono-v1", "trix"),
            ("t-max-400-v1", "tmax"),
            ("trix-400-mono-v1", "trix"),
            ("ultramax-400-ultra-v1", "ultramax"),
            ("velvia-100-v1", "velvia"),
            ("vision3-250d-2383-v1", "vision"),
            ("vision3-500t-supra-v1", "vision"),
            ("vista-200-v1", "vista"),
            ("xp2-super-v1", "xp2"),
            ("xtra-400-crystal-v1", "superia"),
        ];
        let groups = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/src/effectGroups.ts"
        ))
        .unwrap();
        let effects = concat!(env!("CARGO_MANIFEST_DIR"), "/../../effects");
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(effects).unwrap() {
            let path = entry.unwrap().path().join("effect.json");
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let definition: Value = serde_json::from_str(&text).unwrap();
            let id = definition["effect_id"].as_str().unwrap().to_owned();
            let statement = Cover::from_json(&definition["icon"]).unwrap();
            let (_, look) = official
                .iter()
                .find(|(other, _)| *other == id)
                .unwrap_or_else(|| panic!("{id} needs its packagingLook here"));
            let Cover::Film(film) = statement.film_box() else {
                panic!("{id}")
            };
            assert_eq!(film.colors, Some(find_look(look).unwrap().colors()), "{id}");
            assert!(statement.film_box().fits(), "{id}");
            // The service tells a B&W Look by the flag of its file, the site by its group.
            let mono = format!("'{id}': 'mono'");
            assert_eq!(
                definition["flags"] == crate::effects::MONOCHROME,
                groups.contains(&mono),
                "{id}"
            );
            ids.push(id);
        }
        assert_eq!(ids.len(), official.len());

        for (brand, name, layout, number, look) in [
            ("Kodak", "MY LOOK", Layout::Box, None, "kodak"),
            ("HARMAN", "X", Layout::Box, None, "ilford"),
            ("Eastman", "X", Layout::Cine, None, "cinema"),
            ("GRKit", "X", Layout::Stack, None, "gray"),
            ("FUJIFILM", "PROX", Layout::Box, None, "fuji"),
            ("GRKit", "PRO-1", Layout::Box, None, "pro"),
            ("Kodak", "TRI X", Layout::Box, None, "kodak"),
            ("GRKit", "TRIX", Layout::Box, Some("400"), "trix"),
            ("agfa", "X", Layout::Box, None, "agfa"),
            ("GRKit", "X", Layout::Box, Some("5219"), "vision"),
            ("GRKit", "GREEN", Layout::Cine, Some("RICOH"), "cinemaGreen"),
            ("GRKit", "x", Layout::Box, Some("tmax"), "tmax"),
        ] {
            let number = number.unwrap_or("");
            assert_eq!(
                packaging_look(brand, name, layout.template(), number).id,
                look,
                "{brand} {name} {number}"
            );
        }
    }

    #[test]
    fn a_variant_names_the_gray_and_the_film_box_cover() {
        let statement = Cover::for_name("Kodak Portra 400");
        let film_box = statement.film_box();
        assert_eq!(statement.variant_of(&film_box), Some(Variant::FilmBox));
        assert_eq!(
            statement.variant_of_icon(&film_box.pixels().unwrap()),
            Some(Variant::FilmBox)
        );
        let Cover::Film(mut gray) = statement.clone() else {
            unreachable!()
        };
        gray.colors = Some(DEFAULT_COLORS);
        assert_eq!(
            statement.variant_of(&Cover::Film(gray)),
            Some(Variant::Gray)
        );
        let other = Cover::for_name("Kodak Gold 200");
        assert_eq!(statement.variant_of(&other), None);
        assert_eq!(statement.variant_of_icon(&other.pixels().unwrap()), None);
        // An icon that another build drew a little differently counts by the nearer cover.
        for (cover, variant) in [(&statement, Variant::Gray), (&film_box, Variant::FilmBox)] {
            let mut icon = cover.pixels().unwrap();
            icon[..300].iter_mut().for_each(|byte| *byte ^= 1);
            assert_eq!(statement.nearest_variant(&icon), variant);
        }
        // A film box that is gray is the gray cover.
        let plain = Cover::for_name("My Look");
        assert_eq!(plain.variant_of(&plain.film_box()), Some(Variant::Gray));
    }

    #[test]
    fn a_scheme_follows_the_color_and_the_b_w_looks() {
        use Variant::{FilmBox as Box, Gray};
        for (looks, scheme) in [
            (vec![(false, Box), (true, Box)], Scheme::FilmBox),
            (vec![(false, Box), (true, Gray)], Scheme::ColorFilmBox),
            (vec![(false, Box)], Scheme::ColorFilmBox),
            (vec![(false, Box), (false, Gray), (true, Box)], Scheme::Gray),
            (vec![(true, Box)], Scheme::Gray),
            (vec![], Scheme::Gray),
        ] {
            assert_eq!(Scheme::of(looks.clone()), scheme, "{looks:?}");
        }
        // A group without Looks follows the fallback.
        assert_eq!(
            Scheme::of_or(vec![(true, Box)], Scheme::FilmBox),
            Scheme::FilmBox
        );
        assert_eq!(
            Scheme::of_or(vec![(false, Gray)], Scheme::FilmBox),
            Scheme::Gray
        );
        assert_eq!(Scheme::ColorFilmBox.variant(true), Gray);
        assert_eq!(Scheme::ColorFilmBox.variant(false), Box);
    }

    /// The native looks equal `NATIVE_LOOKS` of the site: the same IDs in the same order, the
    /// cover of `cinema` without colors, and its colors as the film box colors.
    #[test]
    fn native_covers_equal_the_site() {
        let source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/src/packageCovers.ts"
        ))
        .unwrap();
        assert!(source.contains(
            "const cinema = (number: string, look: 'cinemaYellow' | 'cinemaGreen'): Cover => ({\n  template: 'cine',\n  brand: 'RICOH',\n  name: 'CINEMA',\n  number,\n  colors: colorsFromLook(look),\n});"
        ));
        let site: Vec<(String, String, String)> = table(&source, "NATIVE_LOOKS", "];")
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("{ id: '")?;
                let (id, rest) = rest.split_once('\'')?;
                let (_, rest) = rest.split_once("cover: cinema('")?;
                let (number, rest) = rest.split_once("', '")?;
                let look = rest.split_once('\'')?.0;
                Some((id.into(), number.into(), look.into()))
            })
            .collect();
        let ours: Vec<_> = crate::gr4_looks::LOOKS
            .iter()
            .filter(|look| look.offered)
            .collect();
        assert_eq!(site.len(), ours.len());
        for (look, (id, number, colors)) in ours.into_iter().zip(&site) {
            assert_eq!(look.effect_id, id);
            let cover = native_cover(id).unwrap();
            let Cover::Film(film) = &cover else {
                unreachable!()
            };
            assert_eq!(
                (film.layout, film.brand.as_str(), film.name.as_str()),
                (Layout::Cine, "RICOH", "CINEMA")
            );
            assert_eq!(film.number.as_deref(), Some(number.as_str()));
            let Cover::Film(film_box) = cover.film_box() else {
                unreachable!()
            };
            assert_eq!(film_box.colors, Some(find_look(colors).unwrap().colors()));
            assert!(cover.fits() && cover.film_box().fits());
        }
        assert_eq!(native_cover("velvia-100-v1"), None);
    }

    #[test]
    fn editor_helpers_are_deterministic() {
        assert_eq!(
            Cover::for_name("Kodak Portra 400").to_json(),
            json!({"template": "box", "brand": "Kodak", "name": "PORTRA", "number": "400", "band": true})
        );
        for name in ["My LUT", "x", "Cinestill 800T", "ABCDEFGHIJKLMNOP"] {
            assert!(Cover::for_name(name).fits(), "{name}");
        }
        let Cover::Film(film) = Cover::for_name("Gold 200") else {
            unreachable!()
        };
        let mut first = film.clone();
        first.shuffle(7);
        let mut again = film.clone();
        again.shuffle(7);
        assert_eq!(first, again);
        for seed in 0..32 {
            let mut shuffled = first.clone();
            shuffled.shuffle(seed);
            assert_ne!(shuffled.layout, first.layout);
            assert_ne!(shuffled.colors, first.colors);
            assert!(EDITOR_LOOKS
                .iter()
                .any(|id| Some(find_look(id).unwrap().colors()) == shuffled.colors));
        }
    }
}
