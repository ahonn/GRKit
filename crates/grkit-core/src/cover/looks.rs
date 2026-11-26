//! The named colors of the covers and the looks of film boxes, as the site has them
//! (`web/src/coverPalette.ts`).

use super::*;

/// Film brands as the catalog covers write them.
pub(super) const BRANDS: [(&str, &str); 9] = [
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
pub const INKS: [(&str, Rgb); 52] = [
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
    ("positiveBody", [12, 28, 52]),
    ("positiveText", [176, 216, 240]),
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
pub const LOOKS: [Look; 36] = [
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
    look(
        "gr2Positive",
        [
            "ricohRed",
            "white",
            "positiveBody",
            "positiveText",
            "chromeBlue",
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
pub const EDITOR_LOOKS: [&str; 36] = [
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
    "gr2Positive",
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
pub(super) const STOCK_LOOKS: [(&str, &str); 30] = [
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
    ("GR II POSITIVE", "gr2Positive"),
    ("CINEMA|RICOH", "ricoh"),
];

/// The look of each film brand (`BRAND_LOOKS`): a regular expression for the brand that
/// ignores case, in the order of the site.
pub(super) const BRAND_LOOKS: [(&str, &str); 4] = [
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
pub(super) fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
