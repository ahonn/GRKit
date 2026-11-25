use super::{looks::*, svg::*, *};
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

/// The text of each drawn text of an SVG document, which must be well-formed XML.
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
            Node::Element(element) if element.name == "path" => {
                element.attr("aria-label").map(str::to_owned)
            }
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
        assert!(!svg.contains("<text"));
    }
    // A large name this wide gets smaller.
    assert!(large_size("LONGER NAMES") < LARGE_NAME.size);
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
        Scheme::of_or(vec![], Scheme::NEW_CARD),
        Scheme::ColorFilmBox
    );
    assert_eq!(
        Scheme::of_or(vec![(false, Gray)], Scheme::NEW_CARD),
        Scheme::Gray
    );
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
    let looks = source.split_once("export const NATIVE_LOOKS").unwrap().1;
    // The Cinema entries of the table, each an object of its own lines.
    let site: Vec<(String, String, String)> = table(looks, "[] = [", "];")
        .split("\n  {\n")
        .filter_map(|entry| {
            let (_, rest) = entry.split_once("id: '")?;
            let id = rest.split_once('\'')?.0;
            let (_, rest) = entry.split_once("cover: cinema('")?;
            let (number, rest) = rest.split_once("', '")?;
            let look = rest.split_once('\'')?.0;
            Some((id.into(), number.into(), look.into()))
        })
        .collect();
    let ours: Vec<_> = crate::gr4_looks::LOOKS
        .iter()
        .filter(|look| matches!(look.kind, crate::gr4_looks::Kind::Color))
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
