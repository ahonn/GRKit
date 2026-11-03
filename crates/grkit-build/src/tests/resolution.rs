use super::*;

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4 and ARM toolchain"]
fn gr4_relocation_resolves_other_builds() -> Result<()> {
    let firmware = PathBuf::from(
        std::env::var("GRKIT_TEST_FIRMWARE_GR4").context("Set GRKIT_TEST_FIRMWARE_GR4")?,
    );
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR4_111)?;
    let directory = tempfile::tempdir()?;
    let built = build_image(&body, &directory.path().join("gr4"), &tools)?;
    // The pinned image resolves each symbol to its pinned address, so the
    // installer changes nothing on it.
    let (result, symbols) = resolve_image(&built.output, &body.body)?;
    assert_eq!(result, "ok 0x543d3000");
    assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    // GR IV 1.04, another build with other addresses, if given.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR4_104") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, symbols) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert_eq!(result, "ok 0x543d2000");
        let resolved: BTreeMap<_, _> = symbols.into_iter().collect();
        // FW_SETTINGS, a template getter found through its caller, and the
        // capture property singleton in the BSS.
        assert_eq!(resolved[&0x5323f018], 0x5323ef40);
        assert_eq!(resolved[&0x533b7110], 0x533b7004);
        assert_eq!(resolved[&0x5537960c], 0x553795fc);
    }
    // GR IV HDF also reports 1.11 for the February 18 build from issue #9.
    // This is a complete, unpatched RTOS capture, not an update container.
    // Keep the input local; these expected addresses were checked independently
    // while preparing a fixed-address runtime tested on that camera.
    if let Some(path) = std::env::var_os("GRKIT_TEST_RTOS_GR4_111_20260218") {
        let image = fs::read(path)?;
        ensure!(
            image.len() == 0x013d4140
                && firmware::sha256(&image)
                    == "60250bf7c497af13d6579fc58736cfddbffa32ffb4f8ff3a1403854b9da40d5e",
            "Expected the complete, unpatched GR IV HDF February 18 RTOS capture"
        );
        let (result, symbols) = resolve_image(&built.output, &image)?;
        assert_eq!(result, "ok 0x543d5000");
        let resolved: BTreeMap<_, _> = symbols.into_iter().collect();
        for (pinned, expected) in [
            (0x543d3000, 0x543d5000), // Tail after the larger image.
            (0x553b2ab0, 0x553b2ae0), // Script array count and table.
            (0x553b3ec4, 0x553b3ef4),
            (0x558372d4, 0x55837314), // BSS end / minimum heap address.
            (0x531bf9d8, 0x531bfad0), // ADJ touch and icon lookup hooks.
            (0x5323db68, 0x5323dc60),
            (0x533b7110, 0x533b72f0), // Parameter getter found through a caller.
            (0x5537960c, 0x5537962c), // Settings singleton.
            (0x5500211c, 0x5500211c), // English slot does not move.
            (0x5501adbc, 0x5501adc4), // Standard record and native tables.
            (0x55053780, 0x55053788),
            (0x55053504, 0x5505350c),
            (0x55052cf4, 0x55052cfc),
            (0x540498d8, 0x5404a888), // Four initialized Standard-record pointers.
            (0x54049a70, 0x5404aa20),
            (0x54049b28, 0x5404aad8),
            (0x54049518, 0x5404a4c8),
        ] {
            assert_eq!(
                resolved[&pinned], expected,
                "February-build relocation of {pinned:#x}"
            );
        }
    }
    // GR III is not a build of this version: nothing resolves.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, _) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert!(result.starts_with("failed"));
    }
    Ok(())
}

/// A GR IV package runs the installer of each GR IV build in turn
/// (`runtime::package_runtime_files`). Each image names exactly one runtime
/// with its identity word (`--known`), and each runtime resolves only its own
/// image (`--try`), so the package installs the runtime of the build.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120, GRKIT_TEST_FIRMWARE_GR4_120_0528, and ARM toolchain"]
fn gr4_installers_select_their_build() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let mut builds = Vec::new();
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
        ("GRKIT_TEST_FIRMWARE_GR4_120_0528", &target::GR4_120_0528),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let built = build_image(&image, &directory.path().join(target.name), &tools)?;
        builds.push((target, image, built));
    }
    let offset = (target::RTOS_IDENTITY - target::RTOS_BASE) as usize;
    for (target, image, _) in &builds {
        let identity = u32::from_le_bytes(image.body[offset..offset + 4].try_into()?);
        let naming: Vec<_> = builds
            .iter()
            .filter(|(other, _, _)| other.identities.contains(&identity))
            .map(|(other, _, _)| other.name)
            .collect();
        assert_eq!(naming, [target.name], "identity {identity:#010x}");
    }
    for (target, _, built) in &builds {
        for (other, image, _) in &builds {
            let (result, symbols) = resolve_image(&built.output, &image.body)?;
            if target.name == other.name {
                let tail = target.layout.tail;
                assert_eq!(result, format!("ok {tail:#x}"), "{}", target.name);
                assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
            } else {
                assert!(
                    result.starts_with("failed"),
                    "{} on {}: {result}",
                    target.name,
                    other.name
                );
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE and ARM toolchain"]
fn gr3_relocation_resolves_its_builds() -> Result<()> {
    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR3_210)?;
    let directory = tempfile::tempdir()?;
    let built = build_image(&body, &directory.path().join("gr3"), &tools)?;
    let (result, symbols) = resolve_image(&built.output, &body.body)?;
    assert_eq!(result, "ok 0x540b6000");
    assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    // GR IIIx 1.60 runs the RTOS code of GR III 2.10, if given.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR3X") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, symbols) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert_eq!(result, "ok 0x540b6000");
        assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    }
    // GR IV is not a build of this version: nothing resolves.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR4") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, _) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert!(result.starts_with("failed"));
    }
    Ok(())
}

/// An RTOS image of `size` bytes whose startup literals give `.data` at
/// `data_source` with `data_size` bytes, and `words` at their addresses.
fn rtos_image(size: usize, data_source: u32, data_size: u32, words: &[(u32, u32)]) -> Vec<u8> {
    let mut image = vec![0; size];
    let mut put = |address: u32, word: u32| {
        let at = (address - target::RTOS_BASE) as usize;
        image[at..at + 4].copy_from_slice(&word.to_le_bytes());
    };
    let literals = [
        data_source,
        relocation::DATA_START,
        data_size,
        0x5510_0000,
        0x5520_0000,
    ];
    for (index, word) in literals.into_iter().enumerate() {
        put(relocation::STARTUP_LITERALS + index as u32 * 4, word);
    }
    for &(address, word) in words {
        put(address, word);
    }
    image
}

#[test]
fn resolution_finds_moved_windows_and_needs_agreeing_sources() -> Result<()> {
    use relocation::{mask, Plan, Region, Source, Symbol, Window};
    // A function at 0x53001000 in the plan, at 0x53041000 in the other
    // image: it moved by four times the installer radius. Its words: a
    // prologue, a BL to 0x53001100 (+0x40000), a literal, MOVW and MOVT of
    // 0x5537abcd, and a return.
    let delta = 0x40000;
    let function = |base: u32, moved: u32| {
        vec![
            (base, 0xe92d_4010),
            (base + 4, 0xeb00_003d), // BL base + 0x100
            (base + 8, 0x5300_1234 + moved),
            (base + 12, 0xe30a_0bcd), // MOVW r0, #0xabcd
            (base + 16, 0xe345_0537), // MOVT r0, #0x5537
            (base + 20, 0xe8bd_8010),
        ]
    };
    let words: Vec<u32> = function(0x5300_1000, 0)
        .iter()
        .map(|&(_, word)| mask(word))
        .collect();
    let other = rtos_image(
        0x60000,
        0x5305_0000,
        0x100,
        &function(0x5300_1000 + delta, delta),
    );
    let window = |start, words: &[u32]| Window {
        region: Region::Image,
        start,
        words: words.to_vec(),
        search: true,
        probe: 0,
    };
    // Window 1 has code that the other image lacks.
    let missing = [words.as_slice(), &[0xe1a0_0000]].concat();
    let symbol = |address, sources| Symbol { address, sources };
    let plan = Plan {
        windows: vec![window(0x5300_1000, &words), window(0x5300_2000, &missing)],
        symbols: vec![
            symbol(
                0x5300_1008,
                vec![Source::Window {
                    window: 0,
                    offset: 8,
                }],
            ),
            symbol(
                0x5300_1100,
                vec![Source::Branch {
                    window: 0,
                    index: 1,
                    verify: None,
                }],
            ),
            symbol(
                0x5300_1234,
                vec![Source::Literal {
                    window: 0,
                    index: 2,
                }],
            ),
            symbol(
                0x5537_abcd,
                vec![Source::Pair {
                    window: 0,
                    movw: 3,
                    movt: 4,
                }],
            ),
            // A second source that gives another address.
            symbol(
                0x5300_1004,
                vec![
                    Source::Window {
                        window: 0,
                        offset: 4,
                    },
                    Source::Window {
                        window: 0,
                        offset: 8,
                    },
                ],
            ),
            // A missing window gives no value; the other source gives it.
            symbol(
                0x5300_1000,
                vec![
                    Source::Window {
                        window: 1,
                        offset: 0,
                    },
                    Source::Window {
                        window: 0,
                        offset: 0,
                    },
                ],
            ),
            symbol(
                0x5300_2000,
                vec![Source::Window {
                    window: 1,
                    offset: 0,
                }],
            ),
            symbol(0x5305_1000, vec![Source::Tail]),
        ],
    };
    // The installer radius does not reach the moved function.
    let near = relocation::resolve(&plan, &other, relocation::RADIUS)?;
    assert!(near.symbols[0].as_ref().unwrap_err().contains("not found"));
    let far = relocation::resolve(&plan, &other, port::RADIUS)?;
    let value = |index: usize| far.symbols[index].as_ref().map(|(value, _)| *value);
    assert_eq!(value(0), Ok(0x5304_1008));
    assert_eq!(value(1), Ok(0x5304_1100));
    assert_eq!(value(2), Ok(0x5304_1234));
    assert_eq!(value(3), Ok(0x5537_abcd));
    assert!(value(4).unwrap_err().contains("disagree"));
    assert_eq!(far.symbols[5].as_ref().unwrap().1.len(), 1);
    assert_eq!(value(5), Ok(0x5304_1000));
    assert!(value(6).unwrap_err().contains("not found"));
    assert_eq!(value(7), Ok(0x5305_1000));
    assert_eq!(far.layout.tail, 0x5305_1000);
    Ok(())
}
