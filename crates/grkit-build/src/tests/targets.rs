use super::*;

#[test]
fn every_code_region_lies_in_the_tail() {
    for target in target::TARGETS {
        let layout = target.layout;
        let (tail, end) = layout.tail;
        // The decoded image ends where the startup .data copy source ends.
        assert!(
            tail >= target.data_source + target.data_size,
            "{}",
            target.name
        );
        // The processing region, then the menu, then Framing after the menu
        // code, which shares the menu region.
        let (menu_start, menu_stop) = layout.menu();
        let mut next = tail;
        if target.family.processing_source.is_some() {
            let (start, stop) = layout.processing();
            assert!(
                start >= next && start < stop && stop <= menu_start,
                "{}",
                target.name
            );
            next = stop;
        }
        assert!(
            menu_start >= next && menu_start < menu_stop && menu_stop == end,
            "{}",
            target.name
        );
        let (start, stop) = layout.framing(menu_start + 0x10);
        assert!(
            start > menu_start && start % 0x100 == 0 && stop == end,
            "{}",
            target.name
        );
    }
}

#[test]
fn each_camera_offers_what_its_runtime_has() {
    for (camera, name) in grkit_core::camera::CAMERAS
        .into_iter()
        .flat_map(|camera| camera.targets().into_iter().map(move |name| (camera, name)))
    {
        let target = target::find(name).unwrap();
        let layout = target.layout;
        assert_eq!(
            target.family.processing_source.is_some(),
            camera.effects,
            "{}",
            camera.id
        );
        assert!(layout.sites.len() == target.family.hooks.len());
        // Each extension point has hooks: those of the capabilities, and
        // those of the module pages, layers, and filters.
        let points: Vec<_> = target.family.hooks.iter().map(|hook| hook.point).collect();
        for point in grkit_core::capabilities::Point::ALL {
            assert!(points.contains(&point), "{} {point:?}", camera.id);
        }
    }
}

/// The definitions of a target header and of the headers that it includes from
/// `runtime/targets/`, by name. `own` keeps only those of the target header.
fn header_definitions(path: &str, own: bool) -> BTreeMap<String, String> {
    let source = runtime_source(path).unwrap_or_else(|| panic!("No runtime/{path}"));
    let text = std::str::from_utf8(source).unwrap().replace("\\\n", " ");
    let mut definitions = BTreeMap::new();
    for line in text.lines() {
        if let Some(include) = line.strip_prefix("#include \"") {
            if !own {
                let include = include.trim_end_matches('"');
                definitions.extend(header_definitions(&format!("targets/{include}"), false));
            }
        } else if let Some(definition) = line.strip_prefix("#define ") {
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            // Skip the include guards.
            if !(name.starts_with("GRKIT_") && name.ends_with("_H")) {
                definitions.insert(name.to_string(), value.trim().to_string());
            }
        }
    }
    definitions
}

/// Definitions that only some builds of a family have, because the code of
/// those builds differs. The runtime tests each with `#ifdef`.
const BUILD_ONLY_DEFINITIONS: &[(&str, &[&str])] = &[
    (
        "gr4-1.20",
        &["NATIVE_ICON_LIMIT", "NATIVE_MONOCHROME_TYPES_HIGH"],
    ),
    ("gr4-1.20-0528", &["NATIVE_ICON_LIMIT"]),
];

/// The hooks of a build are those of its family (`Family::hooks`). Each build
/// gives one site for each hook, and builds of a family have as many
/// contexts and patches and the same header definitions.
#[test]
fn builds_of_a_family_have_its_hooks_and_the_same_definitions() {
    for target in target::TARGETS {
        let hooks = components::hooks(target).unwrap();
        let symbols: BTreeSet<_> = hooks.iter().map(|hook| hook.spec.symbol).collect();
        assert_eq!(symbols.len(), hooks.len(), "{}", target.family.name);
        let first = target::TARGETS
            .iter()
            .find(|other| other.family.name == target.family.name)
            .unwrap();
        assert!(std::ptr::eq(target.family, first.family), "{}", target.name);
        let (layout, first_layout) = (target.layout, first.layout);
        assert_eq!(
            (
                layout.contexts.len(),
                layout.instruction_patches.len(),
                layout.framing_patches.len()
            ),
            (
                first_layout.contexts.len(),
                first_layout.instruction_patches.len(),
                first_layout.framing_patches.len()
            ),
            "{}",
            target.name
        );
        let names = |target: &target::Target| -> BTreeSet<String> {
            let only: &[&str] = BUILD_ONLY_DEFINITIONS
                .iter()
                .find(|(name, _)| *name == target.name)
                .map_or(&[], |(_, names)| names);
            header_definitions(&target.header(), true)
                .into_keys()
                .filter(|name| !only.contains(&name.as_str()))
                .collect()
        };
        assert_eq!(
            names(target),
            names(first),
            "The header of {} defines other names than that of {}",
            target.name,
            first.name
        );
    }
}

#[test]
fn each_target_has_a_header_a_camera_release_and_an_icon_stride() {
    let selector = std::str::from_utf8(runtime_source("firmware.h").unwrap()).unwrap();
    for target in target::TARGETS {
        let layout = target.layout;
        let define = layout.define.strip_prefix("-D").unwrap();
        assert!(
            selector.contains(&format!(
                "defined({define})\n#include \"{}\"",
                target.header()
            )),
            "runtime/firmware.h does not select {} with {define}",
            target.header()
        );
        // Each build header includes the header of its family, which holds
        // the values that the shared code takes from each family.
        let shared = format!("{}.h", target.family.name);
        let header = runtime_source(&format!("targets/{shared}"))
            .unwrap_or_else(|| panic!("The {} family has no targets/{shared}", target.family.name));
        // `grkit port` reads the addresses of the build header only.
        assert!(
            !std::str::from_utf8(header).unwrap().contains("FIRMWARE(0x"),
            "{shared} names a firmware address; put it in the build header"
        );
        assert!(
            std::str::from_utf8(runtime_source(&target.header()).unwrap())
                .unwrap()
                .contains(&format!("#include \"{shared}\"")),
            "{} does not include {shared}",
            target.header()
        );
        assert!(
            grkit_core::camera::CAMERAS
                .iter()
                .any(|camera| camera.runs(target.name)),
            "No camera release runs {}",
            target.name
        );
        // `grkit_core::runtime::icon_stride` finds the family in the name.
        assert!(
            target.name.starts_with(&format!("{}-", target.family.name)),
            "{}",
            target.name
        );
        let stride = header_definitions(&target.header(), false)
            .remove("GRKIT_ICON_STRIDE")
            .unwrap_or_else(|| panic!("{} defines no GRKIT_ICON_STRIDE", target.name));
        assert_eq!(
            grkit_core::runtime::icon_stride(target.name)
                .unwrap()
                .to_string(),
            stride,
            "{}",
            target.name
        );
    }
    for camera in grkit_core::camera::CAMERAS {
        for release in camera.releases {
            assert!(target::find(release.target).is_ok(), "{}", release.target);
        }
    }
}

/// A header value `FIRMWARE(address)` or `(FIRMWARE(address) + offset)`, as
/// (address, offset).
fn firmware_definition(value: &str) -> Option<(u32, u32)> {
    let value = value.trim_start_matches('(').trim_end_matches(')');
    let (call, offset) = value.split_once(") +").unwrap_or((value, "0"));
    let address = call.strip_prefix("FIRMWARE(0x")?;
    Some((
        u32::from_str_radix(address, 16).ok()?,
        number(offset.trim())?,
    ))
}

/// A decimal or `0x` hexadecimal number of a header value.
fn number(value: &str) -> Option<u32> {
    match value.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex.trim_end_matches('U'), 16).ok(),
        None => value.trim_end_matches('U').parse().ok(),
    }
}

/// Facts that the Rust target and the build header both state agree: the
/// settings domain and the offset of the Image Control type in it, and the
/// hooked functions that the menu code also calls.
#[test]
fn build_headers_agree_with_the_target_tables() {
    for target in target::TARGETS {
        let definitions = header_definitions(&target.header(), false);
        let value = |name: &str| {
            definitions
                .get(name)
                .unwrap_or_else(|| panic!("{} defines no {name}", target.header()))
        };
        let native = target.image_control.unwrap();
        let (address, offset) = firmware_definition(value("FW_SETTINGS_DOMAIN")).unwrap();
        assert_eq!(address + offset, native.settings_domain, "{}", target.name);
        // The header names the address that firmware code loads, as the
        // relocation base of the domain does.
        let base = target
            .relocation_bases
            .iter()
            .find(|(field, _)| *field == native.settings_domain)
            .map_or(native.settings_domain, |&(_, base)| base);
        assert_eq!(address, base, "{}", target.name);
        assert_eq!(
            number(value("SETTINGS_IC_TYPE")),
            Some(native.settings_type),
            "{}",
            target.name
        );
        for (name, symbol) in [
            ("FW_ADJ_REFRESH", "menu_refresh"),
            ("FW_CHANGE_CONTROLLER", "menu_settings_change"),
        ] {
            let site = components::hooks(target)
                .unwrap()
                .into_iter()
                .find(|hook| hook.spec.symbol == symbol)
                .unwrap()
                .address;
            assert_eq!(
                firmware_definition(value(name)),
                Some((site, 0)),
                "{} {name}",
                target.name
            );
        }
    }
}

/// The value that an ARM MOV (immediate) or MOVW loads, and its register.
fn move_value(word: u32) -> Option<(u32, u32)> {
    let register = (word >> 12) & 0xf;
    if word & 0x0ff0_0000 == 0x0300_0000 {
        return Some((register, relocation::move_immediate(word)));
    }
    (word & 0x0fe0_0000 == 0x03a0_0000).then(|| {
        let rotate = ((word >> 8) & 0xf) * 2;
        (register, (word & 0xff).rotate_right(rotate))
    })
}

/// The Fn hint IDs of a build header are those that the native ADJ refresh of
/// its firmware gives to FW_GUIDE_SET_CONTENT: the label in r1, the icon in r3.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn fn_hint_ids_are_those_of_the_native_adj_refresh() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let number = |name: &str| -> Result<u32> {
            let value = definitions[name]
                .trim_start_matches("FIRMWARE(")
                .trim_end_matches(')');
            Ok(u32::from_str_radix(value.trim_start_matches("0x"), 16)?)
        };
        let (setter, text, icon) = (
            number("FW_GUIDE_SET_CONTENT")?,
            number("FN_GUIDE_TEXT")?,
            number("FN_GUIDE_ICON")?,
        );
        let code = &image.body[..(target.data_source - target::RTOS_BASE) as usize];
        let words: Vec<u32> = code
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        let calls = (0..words.len()).filter(|&index| {
            let address = target::RTOS_BASE + index as u32 * 4;
            relocation::branch_target(address, words[index]) == Some(setter)
                && (words[index] >> 24) & 0xf == 0xb
        });
        let matching = calls
            .filter(|&index| {
                let before = &words[index.saturating_sub(9)..index];
                let loads: Vec<_> = before.iter().filter_map(|&word| move_value(word)).collect();
                loads.contains(&(1, text)) && loads.contains(&(3, icon))
            })
            .count();
        assert!(
            matching > 0,
            "{}: no native Fn hint with {text:#x} and {icon:#x}",
            target.name
        );
    }
    Ok(())
}

/// FW_USERDATA_SET of each build header is the Set dispatcher of its
/// firmware, and each setting that recipes use matches the patterns of the
/// reader, with one getter chain and one record shape for the firmware.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn userdata_settings_of_recipes_match_each_firmware() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let dispatcher = definitions["FW_USERDATA_SET"]
            .trim_start_matches("FIRMWARE(")
            .trim_end_matches(')');
        let dispatcher = u32::from_str_radix(dispatcher.trim_start_matches("0x"), 16)?;
        let table = crate::userdata::extract(&image, crate::userdata::recipe_setting)?;
        assert_eq!(table.dispatcher, dispatcher, "{}", target.name);
        // GR IV: the type setter and the model record of Monochrome.
        let address = |name: &str| -> Option<u32> {
            let value = definitions.get(name)?;
            let value = value.trim_start_matches("FIRMWARE(").trim_end_matches(')');
            u32::from_str_radix(value.trim_start_matches("0x"), 16).ok()
        };
        if target.name.starts_with("gr4") {
            let (setter, record) = crate::userdata::monochrome_check(&image)?;
            assert_eq!(
                address("FW_IMAGE_CONTROL_SET"),
                Some(setter),
                "{}",
                target.name
            );
            assert_eq!(address("FW_MODEL_RECORD"), Some(record), "{}", target.name);
        } else {
            assert!(
                address("FW_IMAGE_CONTROL_SET").is_none() && address("FW_MODEL_RECORD").is_none()
            );
        }
        for name in [
            "StillActiveImageControlSet",
            "StillImageControlNo0",
            "StillWhiteBalance",
            "StillWhiteBalanceColorTemp",
            "StillHighlightCorrect",
            "StillShadowCorrect",
            "StillLensShadingCorrection",
            "StillHighIsoNR",
        ] {
            assert!(table.settings.contains_key(name), "{}: {name}", target.name);
        }
        let first = table.settings.values().next().unwrap();
        for (name, setting) in &table.settings {
            assert_eq!(setting.getters, first.getters, "{}: {name}", target.name);
            assert_eq!(
                setting.field.record, first.field.record,
                "{}: {name}",
                target.name
            );
        }
    }
    Ok(())
}

/// runtime/targets/<target>-native.h is the native header of the firmware of
/// each build (`userdata::native_header`). With GRKIT_UPDATE_GENERATED set,
/// the test writes the files instead.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn native_headers_match_each_firmware() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/targets");
    let update = std::env::var_os("GRKIT_UPDATE_GENERATED").is_some();
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let header = crate::userdata::native_header(&image)?;
        let file = root.join(format!("{}-native.h", target.name));
        if update {
            std::fs::write(&file, &header)?;
        } else {
            assert_eq!(std::fs::read_to_string(&file)?, header, "{}", target.name);
        }
    }
    Ok(())
}

/// The native icon of each type (`NATIVE_TYPE_ICONS`), which the native
/// Image Control list shows: Monochrome Standard and Monochrome HDR Cuisine
/// show the icons of Standard and HDR Cuisine, and each other type has its
/// own. On GR III, Standard shows the icon that NATIVE_OFF_ICON reads.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn native_type_icons_match_each_firmware() -> Result<()> {
    use grkit_core::native::TYPES;
    let position = |label: &str| TYPES.iter().position(|t| t.label == label).unwrap();
    let shared = [
        (position("MonochromeStandard"), position("Standard")),
        (position("MonochromeHdrCuisine"), position("HdrCuisine")),
    ];
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let header = crate::userdata::native_header(&image)?;
        let list = |name: &str| -> Vec<u32> {
            let line = header
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .unwrap_or_else(|| panic!("{}: no {name}", target.name));
            line.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        };
        let (codes, icons) = (list("NATIVE_TYPE_CODES"), list("NATIVE_TYPE_ICONS"));
        assert_eq!(icons.len(), TYPES.len(), "{}", target.name);
        for (index, (&code, &icon)) in codes.iter().zip(&icons).enumerate() {
            assert_eq!(
                code == 0xff,
                icon == 0,
                "{}: {}",
                target.name,
                TYPES[index].label
            );
            let twin = shared
                .iter()
                .find(|&&(type_index, _)| type_index == index)
                .map(|&(_, other)| other);
            for (other, &other_icon) in icons.iter().enumerate() {
                if other != index && icon != 0 && other_icon != 0 {
                    assert_eq!(
                        icon == other_icon,
                        twin == Some(other)
                            || shared.iter().any(|&(t, o)| t == other && o == index),
                        "{}: {} and {}",
                        target.name,
                        TYPES[index].label,
                        TYPES[other].label
                    );
                }
            }
        }
        if target.name.starts_with("gr3") {
            let definitions = header_definitions(&target.header(), false);
            assert_eq!(
                definitions["NATIVE_OFF_ICON()"],
                format!("WORD(FW_ICON_TABLE,{:#x}*4)", icons[position("Standard")])
            );
        }
    }
    Ok(())
}

/// Each build header includes its native header, which each build has.
#[test]
fn each_build_includes_its_native_header() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/targets");
    for target in target::TARGETS {
        let header = std::fs::read_to_string(root.join(format!("{}.h", target.name))).unwrap();
        assert!(
            header.contains(&format!("#include \"{}-native.h\"", target.name)),
            "{}",
            target.name
        );
        assert!(root.join(format!("{}-native.h", target.name)).exists());
    }
}

/// The value of an ARM data-processing immediate.
fn immediate(word: u32) -> u32 {
    (word & 0xff).rotate_right(((word >> 8) & 0xf) * 2)
}

/// The keyboard of each build header (runtime/ui.h) is that of its firmware.
/// The length method reads the use byte KEYBOARD_FUNCTION; the hooked load
/// and store pass the same text of the keyboard to their tail calls. GR IV:
/// the keyboard copies the use from FW_KEYBOARD_MODEL at its start, and the
/// Copyright page sets KEYBOARD_COPYRIGHT with FW_KEYBOARD_SET_FUNCTION and
/// opens KEYBOARD_CONTROLLER. GR III: the controller ID KEYBOARD_CONTROLLER
/// starts the keyboard with the use KEYBOARD_COPYRIGHT.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn keyboard_definitions_match_each_firmware() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let value = |name: &str| -> u32 {
            let value = definitions
                .get(name)
                .unwrap_or_else(|| panic!("{} defines no {name}", target.header()));
            firmware_definition(value)
                .map(|(address, _)| address)
                .or_else(|| number(value))
                .unwrap_or_else(|| panic!("{name} = {value}"))
        };
        let word = |address: u32| -> u32 {
            u32::from_le_bytes(image.read(address, 4).unwrap().try_into().unwrap())
        };
        let site = |symbol: &str| -> u32 {
            components::hooks(target)
                .unwrap()
                .into_iter()
                .find(|hook| hook.spec.symbol == symbol)
                .unwrap_or_else(|| panic!("{} has no {symbol}", target.name))
                .address
        };
        let (function, copyright, controller) = (
            value("KEYBOARD_FUNCTION"),
            value("KEYBOARD_COPYRIGHT"),
            value("KEYBOARD_CONTROLLER"),
        );
        // The length method: `mov ip, sp`, then `ldrb r3, [r0, #function]`.
        let length = site("menu_keyboard_length");
        assert_eq!(word(length), 0xe1a0_c00d, "{}", target.name);
        assert!(
            (1..8).any(|k| word(length + 4 * k) == 0xe5d0_3000 | function),
            "{}: the length method reads no use byte",
            target.name
        );
        // `add r1, r4, #a`, `add r1, r1, #b`, then the epilogue and the tail
        // call: the load and the store take the same text.
        let text = |site: u32| -> u32 {
            let (first, second) = (word(site - 16), word(site - 12));
            assert!(
                first & 0xffff_f000 == 0xe284_1000 && second & 0xffff_f000 == 0xe281_1000,
                "{}: {site:#x}",
                target.name
            );
            assert_eq!(word(site) >> 24, 0xea, "{}: {site:#x}", target.name);
            immediate(first) + immediate(second)
        };
        assert_eq!(
            text(site("menu_keyboard_load")),
            text(site("menu_keyboard_store")),
            "{}",
            target.name
        );
        let code = &image.body[..(target.data_source - target::RTOS_BASE) as usize];
        let words: Vec<u32> = code
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        let address = |index: usize| target::RTOS_BASE + index as u32 * 4;
        let calls = |index: usize, callee: u32| {
            (words[index] >> 24) & 0xf == 0xb
                && relocation::branch_target(address(index), words[index]) == Some(callee)
        };
        if definitions.contains_key("FW_KEYBOARD_MODEL") {
            let (model, set) = (
                value("FW_KEYBOARD_MODEL"),
                value("FW_KEYBOARD_SET_FUNCTION"),
            );
            // OnStart: the model getter, its use getter, `strb r0, [r4, #function]`.
            assert!(
                (2..words.len()).any(|i| words[i] == 0xe5c4_0000 | function && calls(i - 2, model)),
                "{}: no keyboard start copies the use of the model",
                target.name
            );
            // The Copyright page: the model, `mov r1, #copyright`, the setter,
            // with the keyboard ID before.
            let pages = (2..words.len())
                .filter(|&i| {
                    calls(i, set)
                        && words[i - 1] == 0xe3a0_1000 | copyright
                        && calls(i - 2, model)
                        && words[i.saturating_sub(16)..i].windows(2).any(|pair| {
                            move_value(pair[0]).map(|(_, low)| low) == Some(controller & 0xffff)
                                && pair[1] & 0xfff0_0fff == 0xe340_0200
                        })
                })
                .count();
            assert!(pages > 0, "{}: no Copyright page sets the use", target.name);
        } else {
            // The start sets the use from the controller ID: `add r1, r1,
            // #0xfe000000`, `sub r1, r1, #first`, `cmp r1, #count`, and a
            // table branch, whose entry for the ID sets `mov r3, #copyright`,
            // `strb r3, [r0, #function]`.
            let starts = (3..words.len())
                .filter(|&i| {
                    if words[i] != 0x979f_f101 || words[i - 8..i].iter().all(|&w| w != 0xe281_14fe)
                    {
                        return false;
                    }
                    let Some(first) = words[i - 8..i]
                        .iter()
                        .find(|&&w| w & 0xffff_f000 == 0xe241_1000)
                        .map(|&w| immediate(w))
                    else {
                        return false;
                    };
                    let Some(entry) = controller.checked_sub(0x0200_0000 + first) else {
                        return false;
                    };
                    let Some(&target_address) = words.get(i + 2 + entry as usize) else {
                        return false;
                    };
                    let Some(index) = target_address
                        .checked_sub(target::RTOS_BASE)
                        .map(|offset| (offset / 4) as usize)
                        .filter(|&index| index + 1 < words.len())
                    else {
                        return false;
                    };
                    words[index] == 0xe3a0_3000 | copyright
                        && words[index + 1] == 0xe5c0_3000 | function
                })
                .count();
            assert_eq!(starts, 1, "{}: the start of the keyboard", target.name);
        }
    }
    Ok(())
}
