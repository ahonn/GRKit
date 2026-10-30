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
                value >> 24 == 0xeb && relocation::branch_target(address, value) == Some(first)
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
        let last = (compare & 0xff).rotate_right(((compare >> 8) & 0xf) * 2);
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
                    .and_then(|(at, value)| relocation::branch_target(at, value))
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
                .then(|| relocation::branch_target(at, value))
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
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, and GRKIT_TEST_FIRMWARE_GR4_120_0528"]
fn parameter_rows_are_those_of_their_row_kinds() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
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
            let call =
                relocation::branch_target(at, words[((at - target::RTOS_BASE) / 4) as usize]);
            assert_eq!(
                call,
                tail_call(&words, getter.unwrap()),
                "{context}: row text"
            );
        }
    }
    Ok(())
}
