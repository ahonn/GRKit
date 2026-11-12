use super::*;

#[test]
fn every_code_region_lies_in_the_tail() -> Result<()> {
    for target in target::TARGETS {
        let layout = target.layout;
        let tail = layout.tail;
        assert!(
            tail >= target.data_source + target.data_size,
            "{}",
            target.name
        );
        let (processing_start, end) = layout.processing()?;
        assert_eq!(processing_start, tail + 0x100);
        assert_eq!(end, components::RTOS_EXECUTABLE_END);
        // Growth beyond the former 1 KiB Processing partition and 48 KiB
        // core budget still leaves room for the following component.
        let processing_end = processing_start + 0x1004;
        let (menu_start, _) = layout.menu(processing_end)?;
        assert_eq!(menu_start, tail + 0x1200);
        let (framing_start, _) = layout.framing(menu_start + 0xc004)?;
        assert_eq!(framing_start, tail + 0xd300);
        assert!(framing_start < end);
        assert!(layout.menu(end).is_err());
        assert!(layout.framing(u32::MAX).is_err());
    }
    Ok(())
}

#[test]
fn core_span_covers_payloads_and_rejects_invalid_bounds() -> Result<()> {
    let tail = target::GR3_210.layout.tail;
    let payload = |address, size, site| Patch {
        address,
        points: 0,
        before: vec![0; size],
        after: vec![0; size],
        site,
    };
    let mut plan = PatchPlan {
        patches: vec![
            payload(tail + 0x100, 0x1004, Some(Anchor::Tail)),
            payload(tail + 0x1200, 0xc004, Some(Anchor::Tail)),
        ],
        relocations: vec![],
        references: vec![],
        exports: vec![],
        viewfinder: 0,
    };
    assert_eq!(plan.core_size(tail)?, 0xd300);
    validate_plan(&plan)?;
    plan.patches
        .push(payload(tail + 0x1200, 4, Some(Anchor::Tail)));
    assert!(validate_plan(&plan).is_err());
    plan.patches.pop();
    // A firmware hook does not change the core high-water mark.
    plan.patches
        .push(payload(tail - 4, 4, Some(Anchor::Firmware(tail - 4))));
    assert_eq!(plan.core_size(tail)?, 0xd300);
    for patch in [
        payload(tail - 4, 4, Some(Anchor::Tail)),
        payload(components::RTOS_EXECUTABLE_END - 4, 8, Some(Anchor::Tail)),
        payload(u32::MAX - 3, 8, Some(Anchor::Tail)),
        payload(tail + 0x100, 4, None),
        payload(tail - 4, 8, None),
    ] {
        plan.patches = vec![patch];
        assert!(plan.core_size(tail).is_err());
    }
    plan.patches = vec![payload(
        components::RTOS_EXECUTABLE_END - 4,
        4,
        Some(Anchor::Tail),
    )];
    assert_eq!(
        tail + plan.core_size(tail)?,
        components::RTOS_EXECUTABLE_END
    );
    plan.patches.clear();
    assert_eq!(plan.core_size(tail)?, 0);
    Ok(())
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
        for point in grkit_core::capabilities::Point::ALL {
            assert!(
                target
                    .family
                    .hooks
                    .iter()
                    .any(|hook| hook.point_mask() & point.bit() != 0),
                "{} {point:?}",
                camera.id
            );
        }
    }
}

#[test]
fn framing_or_border_without_film_look_still_records_shot_parameters() {
    use crate::components::{Part, Point};
    for target in target::TARGETS {
        let hook = target
            .family
            .hooks
            .iter()
            .find(|hook| hook.symbol == "module_parameters")
            .unwrap();
        assert_eq!(hook.part, Part::Processing);
        assert!(hook.link);
        assert_eq!(
            hook.point_mask(),
            Point::Tuning.bit() | Point::CaptureAspect.bit() | Point::PictureCanvas.bit()
        );
        // The RAW development lists the effects and the Framing ratios with
        // the same list hooks, and clears both at its start.
        const DEVELOP: [&str; 5] = [
            "picture_develop_tone",
            "menu_develop_count",
            "menu_develop_row",
            "menu_develop_current",
            "menu_develop_open",
        ];
        for other in target
            .family
            .hooks
            .iter()
            .filter(|hook| hook.symbol != "module_parameters")
        {
            let expected = if DEVELOP.contains(&other.symbol) {
                Point::Tuning.bit() | Point::CaptureAspect.bit()
            } else if other.symbol == "grain_gate" {
                // Film Look gives the gate the Shading and Clarity of its effect.
                Point::PipelineGrain.bit() | Point::Tuning.bit()
            } else {
                other.point.bit()
            };
            assert_eq!(other.point_mask(), expected, "{}", other.symbol);
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
    (
        "gr4-1.21",
        &["NATIVE_ICON_LIMIT", "NATIVE_MONOCHROME_TYPES_HIGH"],
    ),
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

/// A value that each build of a family has is a value of the family: it is
/// in the family header, so that a change and a port touch it once. A build
/// header keeps its addresses and the values that differ in its build.
#[test]
fn values_that_each_build_of_a_family_shares_are_in_the_family_header() {
    let mut families: BTreeMap<&str, Vec<&target::Target>> = BTreeMap::new();
    for target in target::TARGETS {
        families.entry(target.family.name).or_default().push(target);
    }
    for (family, builds) in families {
        if builds.len() < 2 {
            continue;
        }
        let headers: Vec<_> = builds
            .iter()
            .map(|build| header_definitions(&build.header(), true))
            .collect();
        let shared: Vec<&String> = headers[0]
            .iter()
            .filter(|(name, value)| {
                !value.contains("FIRMWARE(")
                    && headers[1..]
                        .iter()
                        .all(|other| other.get(*name) == Some(*value))
            })
            .map(|(name, _)| name)
            .collect();
        assert!(
            shared.is_empty(),
            "Each {family} build header defines {shared:?} with the same value: move them to \
             runtime/targets/{family}.h"
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
        return Some((register, arm::move_immediate(word)));
    }
    (word & 0x0fe0_0000 == 0x03a0_0000).then(|| (register, arm::modified_immediate(word)))
}

/// The Snap focus distance texts of a build header (`runtime/distance.h`) are
/// those of the English table of its firmware: 0.3 m to 5 m.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn snap_distance_texts_are_those_of_each_firmware() -> Result<()> {
    const DISTANCES: [&str; 8] = ["0.3m", "0.4m", "1m", "1.5m", "2m", "2.5m", "3.5m", "5m"];
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let first = number(&definitions["SNAP_DISTANCE_TEXT"]).context("SNAP_DISTANCE_TEXT")?;
        let word = |address: u32| -> Result<u32> {
            Ok(u32::from_le_bytes(image.read(address, 4)?.try_into()?))
        };
        let table = word(target.english_slot)?;
        for (index, distance) in DISTANCES.iter().enumerate() {
            let descriptor = word(table + 4 * (first + index as u32))?;
            let (header, text) = (word(descriptor)?, word(descriptor + 4)?);
            let units: Vec<u16> = image
                .read(text, 2 * distance.len())?
                .chunks_exact(2)
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                .collect();
            assert_eq!(
                (header, String::from_utf16(&units)?.as_str()),
                (0x100 + distance.len() as u32 + 1, *distance),
                "{}: text {}",
                target.name,
                first + index as u32
            );
        }
    }
    Ok(())
}

/// The scale hooks (`runtime/distance.h`) replace the BLs to
/// FW_ICON_VIEW_SET_ICON of the normal manual focus scale, one after the
/// other: the unit, infinity, then 5, 3, 1, and 0.3 m. The 1 m and 3 m labels
/// set SCALE_ICON_ONE and SCALE_ICON_THREE, and the macro scale later in the
/// same function sets SCALE_ICON_TEN.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn scale_sites_and_icons_are_those_of_each_firmware() -> Result<()> {
    const SITES: [&str; 5] = [
        "menu_scale_meters",
        "menu_scale_five",
        "menu_scale_three",
        "menu_scale_one",
        "menu_scale_tenth",
    ];
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let value = |name: &str| -> u32 {
            let value = &definitions[name];
            firmware_definition(value)
                .map(|(address, _)| address)
                .or_else(|| number(value))
                .unwrap_or_else(|| panic!("{name} = {value}"))
        };
        let word = |address: u32| -> u32 {
            u32::from_le_bytes(image.read(address, 4).unwrap().try_into().unwrap())
        };
        let set_icon = value("FW_ICON_VIEW_SET_ICON");
        let sets = |address: u32| arm::branch_target(address, word(address)) == Some(set_icon);
        let icon = |address: u32| {
            move_value(word(address - 4))
                .filter(|&(r, _)| r == 1)
                .map(|(_, id)| id)
        };
        let hooks = components::hooks(target)?;
        let sites: Vec<u32> = SITES
            .iter()
            .map(|symbol| {
                hooks
                    .iter()
                    .find(|hook| hook.spec.symbol == *symbol)
                    .unwrap_or_else(|| panic!("{} has no {symbol}", target.name))
                    .address
            })
            .collect();
        assert_eq!(
            sites
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .collect::<Vec<_>>(),
            [24, 12, 12, 12],
            "{}",
            target.name
        );
        assert!(sites.iter().all(|&site| sets(site)), "{}", target.name);
        assert!(sets(sites[0] + 12), "{}: infinity", target.name);
        assert_eq!(
            icon(sites[3]),
            Some(value("SCALE_ICON_ONE")),
            "{}",
            target.name
        );
        assert_eq!(
            icon(sites[2]),
            Some(value("SCALE_ICON_THREE")),
            "{}",
            target.name
        );
        let ten = value("SCALE_ICON_TEN");
        assert!(
            (sites[4]..sites[4] + 0x200)
                .step_by(4)
                .any(|address| sets(address) && icon(address) == Some(ten)),
            "{}: no SetIcon of {ten:#x}",
            target.name
        );
        // The icon setup is the function of the sites, and the update entry
        // calls it after it stores the macro flag of the scale.
        let setup = value("FW_FOCUS_SCALE_ICONS");
        let site = |symbol: &str| {
            hooks
                .iter()
                .find(|hook| hook.spec.symbol == symbol)
                .unwrap_or_else(|| panic!("{} has no {symbol}", target.name))
                .address
        };
        assert_eq!(word(setup), 0xe1a0_c00d, "{}", target.name);
        assert!(sites[0] - setup < 0x80, "{}", target.name);
        let update = site("menu_scale_update_entry");
        assert!(
            (update..update + 0x100).step_by(4).any(|address| {
                arm::branch_target(address, word(address)) == Some(setup)
                    && word(address - 8) == 0xe5c4_0124
            }),
            "{}: the update does not set the icons",
            target.name
        );
        // The text setter entry is the string view setter of the header.
        assert_eq!(
            site("menu_string_text_entry"),
            value("FW_STRING_VIEW_RESET"),
            "{}",
            target.name
        );
    }
    Ok(())
}

/// The Fn hint IDs of a build header are those that the native ADJ refresh of
/// its firmware gives to FW_GUIDE_SET_CONTENT: the label in r1, the icon in r3.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn fn_hint_ids_are_those_of_the_native_adj_refresh() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
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
            arm::branch_target(address, words[index]) == Some(setter)
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

/// On GR IV the model setters of the parameter page are wrappers of 0x50 bytes,
/// one for each row kind, from that of kind 0. The setter site of each row is
/// the wrapper of its row kind (`PARAMETER_KINDS`). GR III does not space its
/// setters evenly, so it is not checked here.
#[test]
fn gr4_parameter_setters_are_those_of_their_row_kinds() {
    for target in target::TARGETS
        .iter()
        .filter(|target| target.family.name == "gr4")
    {
        let definitions = header_definitions(&target.header(), false);
        let kinds: Vec<u32> = definitions["PARAMETER_KINDS"]
            .trim_matches(|c| c == '{' || c == '}')
            .split(',')
            .map(|kind| kind.trim().parse().unwrap())
            .collect();
        let site = |symbol: &str| {
            target
                .layout
                .sites
                .iter()
                .find(|site| site.0 == symbol)
                .unwrap_or_else(|| panic!("{} has no {symbol}", target.name))
                .1
        };
        let first = site("menu_parameter_set_0");
        for (row, kind) in kinds.iter().enumerate() {
            assert_eq!(
                site(&format!("menu_parameter_set_{row}")),
                first + kind * 0x50,
                "{} row {row} (kind {kind})",
                target.name
            );
        }
    }
}

/// The function of each row kind in the parameter model, by row kind. The
/// model selects the function of a row kind from a table of cases. The table
/// is found near a call of `first`, the function of kind 0, and each case
/// calls the function of its kind first.
fn kind_functions(words: &[u32], first: u32) -> Option<Vec<Option<u32>>> {
    let word = |address: u32| {
        words
            .get(((address - target::RTOS_BASE) / 4) as usize)
            .copied()
    };
    let calls = (0..words.len() as u32)
        .map(|index| target::RTOS_BASE + index * 4)
        .filter(|&address| {
            word(address).is_some_and(|value| {
                value >> 24 == 0xeb && arm::branch_target(address, value) == Some(first)
            })
        });
    for call in calls {
        let Some(switch) = (0..0x200u32)
            .map(|step| call - step * 4)
            .find(|&at| word(at).is_some_and(|value| value & 0xffff_fff0 == 0x979f_f100))
        else {
            continue;
        };
        // The largest row kind of the table.
        let compare = word(switch - 4)?;
        if compare & 0x0ff0_f000 != 0x0350_0000 {
            continue;
        }
        let last = arm::modified_immediate(compare);
        let kinds = (0..=last)
            .map(|kind| {
                let case = word(switch + 8 + kind * 4)?;
                (0..16)
                    .map(|step| case + step * 4)
                    .map_while(|at| {
                        word(at)
                            .filter(|value| value >> 24 != 0xea)
                            .map(|v| (at, v))
                    })
                    .find(|(_, value)| value >> 24 == 0xeb)
                    .and_then(|(at, value)| arm::branch_target(at, value))
            })
            .collect();
        return Some(kinds);
    }
    None
}

/// The native getter that a getter of the parameter model passes on to: its
/// first jump to a distant function.
fn tail_call(words: &[u32], function: u32) -> Option<u32> {
    (0..64)
        .map(|step| function + step * 4)
        .filter_map(|at| {
            let value = *words.get(((at - target::RTOS_BASE) / 4) as usize)?;
            (value >> 24 == 0xea)
                .then(|| arm::branch_target(at, value))
                .flatten()
        })
        .find(|&to| to.abs_diff(function) > 0x1000)
}

/// Each row of the parameter page reads, writes, and names the row kind of
/// `PARAMETER_KINDS`: its getter and setter sites are the functions that the
/// jump tables of the parameter model call for that kind, and its row site
/// calls the native getter of that kind. GR IV 1.20 once had the setters of
/// shading and clarity one kind too far, and the stock bytes could not show
/// it, because those setters are equal functions.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn parameter_rows_are_those_of_their_row_kinds() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let words: Vec<u32> = image.body[..(target.data_source - target::RTOS_BASE) as usize]
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        let definitions = header_definitions(&target.header(), false);
        let kinds: Vec<u32> = definitions["PARAMETER_KINDS"]
            .trim_matches(|c| c == '{' || c == '}')
            .split(',')
            .map(|kind| kind.trim().parse().unwrap())
            .collect();
        let site = |symbol: &str| -> u32 {
            target
                .layout
                .sites
                .iter()
                .find(|site| site.0 == symbol)
                .unwrap_or_else(|| panic!("{} has no {symbol}", target.name))
                .1
        };
        let getters = kind_functions(&words, site("menu_parameter_get_0"))
            .with_context(|| format!("{}: no getter switch", target.name))?;
        let setters = kind_functions(&words, site("menu_parameter_set_0"))
            .with_context(|| format!("{}: no setter switch", target.name))?;
        for (row, &kind) in kinds.iter().enumerate() {
            let getter = getters.get(kind as usize).copied().flatten();
            let setter = setters.get(kind as usize).copied().flatten();
            let context = format!("{} row {row} (kind {kind})", target.name);
            assert_eq!(
                Some(site(&format!("menu_parameter_get_{row}"))),
                getter,
                "{context}: getter"
            );
            assert_eq!(
                Some(site(&format!("menu_parameter_set_{row}"))),
                setter,
                "{context}: setter"
            );
            let at = site(&format!("menu_parameter_row_{row}"));
            let call = arm::branch_target(at, words[((at - target::RTOS_BASE) / 4) as usize]);
            assert_eq!(
                call,
                tail_call(&words, getter.unwrap()),
                "{context}: row text"
            );
        }
    }
    Ok(())
}

/// The GR IV sources of `grkit_core::gr4_looks`, which the site reads without the build
/// targets, name the same files, data sections, and type tables as the targets.
#[test]
fn gr4_look_sources_match_their_targets() {
    for source in &grkit_core::gr4_looks::SOURCES {
        let target = target::TARGETS
            .iter()
            .find(|t| t.firmware_sha256 == source.firmware_sha256)
            .unwrap_or_else(|| panic!("No target for GR IV {}", source.version));
        assert_eq!(target.firmware, format!("GR IV {}", source.version));
        let tables = target.image_control.unwrap().tables;
        assert_eq!(
            source.layout(),
            (
                target.data_start,
                target.data_source,
                target.data_size,
                [tables[0], tables[1]]
            ),
            "{}",
            target.name
        );
    }
}

/// Each firmware address of the runtime is in the header of a build, so a new
/// build of a family gives each one (and `grkit port` finds each one). Code
/// that a family shares names an address only by its definition. The fast-boot
/// targets are build headers of their own.
#[test]
fn shared_runtime_code_names_no_firmware_address() {
    for (name, bytes) in crate::RUNTIME_SOURCES {
        if name.starts_with("targets/") || name.starts_with("fastboot/targets/") {
            continue;
        }
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains("FIRMWARE(0x"),
            "{name} names a firmware address outside the build headers"
        );
    }
}

/// REPORT.TXT names the camera of each identity that a build knows: one camera
/// runs the build at the version of the identity (`loader::header`).
#[test]
fn each_identity_has_one_camera() {
    for target in target::TARGETS {
        for &identity in target.identities {
            let version = target::identity_version(identity);
            let cameras: Vec<_> = grkit_core::camera::CAMERAS
                .iter()
                .filter(|camera| camera.firmware_for(target.name).contains(&version.as_str()))
                .map(|camera| camera.name)
                .collect();
            assert_eq!(
                cameras.len(),
                1,
                "{} {identity:#010x} {version}: {cameras:?}",
                target.name
            );
        }
    }
    assert_eq!(target::identity_version(0x0b0a1401), "1.20");
    assert_eq!(target::identity_version(0x12144601), "1.70");
}

/// FW_USERDATA_SET of each build header is the Set dispatcher of its
/// firmware, and each setting that recipes use matches the patterns of the
/// reader, with one getter chain and one record shape for the firmware.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn userdata_settings_of_recipes_match_each_firmware() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
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
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn native_headers_match_each_firmware() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/targets");
    let update = std::env::var_os("GRKIT_UPDATE_GENERATED").is_some();
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
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
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn native_type_icons_match_each_firmware() -> Result<()> {
    use grkit_core::native::TYPES;
    let position = |label: &str| TYPES.iter().position(|t| t.label == label).unwrap();
    let shared = [
        (position("MonochromeStandard"), position("Standard")),
        (position("MonochromeHdrCuisine"), position("HdrCuisine")),
    ];
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
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

/// The keyboard of each build header (runtime/ui_keyboard.h) is that of its firmware.
/// The length method reads the use byte KEYBOARD_FUNCTION; the hooked load
/// and store pass the same text of the keyboard to their tail calls. GR IV:
/// the keyboard copies the use from FW_KEYBOARD_MODEL at its start, and the
/// Copyright page sets KEYBOARD_COPYRIGHT with FW_KEYBOARD_SET_FUNCTION and
/// opens KEYBOARD_CONTROLLER. GR III: the controller ID KEYBOARD_CONTROLLER
/// starts the keyboard with the use KEYBOARD_COPYRIGHT.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121"]
fn keyboard_definitions_match_each_firmware() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR3_221", &target::GR3_221),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
        ("GRKIT_TEST_FIRMWARE_GR4_121", &target::GR4_121),
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
            arm::modified_immediate(first) + arm::modified_immediate(second)
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
                && arm::branch_target(address(index), words[index]) == Some(callee)
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
                        .map(|&w| arm::modified_immediate(w))
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
