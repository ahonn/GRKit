use super::*;

fn synthetic_firmware() -> firmware::FirmwareImage {
    let gr3 = &target::GR3_210;
    let mut image = firmware::FirmwareImage {
        target: gr3,
        body: vec![0; (gr3.data_source - target::RTOS_BASE + gr3.data_size) as usize],
        sha256: "synthetic".into(),
        decoded_sha256: "synthetic".into(),
    };
    let mut write = |address: u32, bytes: &[u8]| {
        let address = if (gr3.data_start..gr3.data_start + gr3.data_size).contains(&address) {
            address - gr3.data_start + gr3.data_source
        } else {
            address
        };
        let offset = (address - target::RTOS_BASE) as usize;
        image.body[offset..offset + bytes.len()].copy_from_slice(bytes);
    };
    for &(address, bytes) in components::CONTEXTS {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, _, bytes) in components::MENU_HOOKS {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, _, bytes, _) in components::FRAMING_HOOKS
        .iter()
        .chain(components::VIEWFINDER_HOOKS.iter())
        .chain(components::METADATA_HOOKS.iter())
        .chain(components::PICTURE_HOOKS.iter())
    {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, bytes, _) in components::INSTRUCTION_PATCHES
        .iter()
        .chain(components::FRAMING_PATCHES.iter())
    {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, bytes) in components::GR3_LAYOUT.guards {
        write(address, &hex::decode(bytes).unwrap());
    }
    for address in target::GR3_IMAGE_CONTROL.tables {
        write(address, &(0..104u8).collect::<Vec<_>>());
    }
    write(
        target::GR3_IMAGE_CONTROL.tables[0] + 11 * 4,
        &0x5501900cu32.to_le_bytes(),
    );
    write(0x5501900c, &(0..84u8).collect::<Vec<_>>());
    write(0x55019010, &0x53dc2754u32.to_le_bytes());
    // The English entry of the language table, as GR III 2.10 holds it.
    write(gr3.english_slot, &0x55004490u32.to_le_bytes());
    image
}

#[test]
fn decoder_rejects_truncation_and_supports_overlap() {
    for tail in [
        vec![],
        vec![0x80, 0x10],
        vec![0, 4, 0x80, 0, 8, 1],
        vec![0, 1, 0],
    ] {
        let mut blob = vec![0; 128];
        blob.extend(tail);
        assert!(firmware::decode(&blob).is_err());
    }
    let mut blob = vec![0; 128];
    blob.extend_from_slice(b"\x80\x03abc\x00\x00");
    assert_eq!(firmware::decode(&blob).unwrap(), b"abc");
    let mut overlap = vec![0; 128];
    overlap.extend_from_slice(b"\x00\x05\x40\x00a\x03\x01\x00\x00");
    assert_eq!(firmware::decode(&overlap).unwrap(), b"aaaaaaa");
}

#[test]
fn firmware_identity_rejected_before_decode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("firmware.bin");
    fs::write(&path, b"wrong firmware").unwrap();
    assert!(verify_firmware(&path, &target::GR3_210)
        .unwrap_err()
        .to_string()
        .contains("SHA-256"));
}

#[test]
fn firmware_address_mapping_and_record_contract() {
    let mut body = synthetic_firmware();
    assert_eq!(body.stock_record().unwrap().0, 0x5501900c);
    assert_eq!(
        body.read(target::GR3_210.data_start, 8).unwrap(),
        body.read(target::GR3_210.data_source, 8).unwrap()
    );
    assert!(body.read(0, 4).is_err());
    assert!(body.read(target::RTOS_BASE, usize::MAX).is_err());
    let offset = (target::GR3_210.data_source - target::RTOS_BASE + 0x19010) as usize;
    body.body[offset] ^= 1;
    assert!(body.stock_record().is_err());
}

#[test]
fn patch_and_branch_validation() {
    assert_eq!(arm::branch(0x1000, 0x1008, 14).unwrap(), [0, 0, 0, 0xea]);
    for target in [0x1009, 0x3000000] {
        assert!(arm::branch(0x1000, target, 14).is_err());
    }
    assert!(arm::elf_text(&[0; 64]).is_err());
    let patch = Patch {
        address: 0x1000,
        before: vec![0; 4],
        after: target::HEADER_REFERENCE.to_le_bytes().to_vec(),
    };
    let mut plan = PatchPlan {
        patches: vec![patch.clone()],
        relocations: vec![(0, 0)],
    };
    validate_plan(&plan).unwrap();
    for relocations in [
        vec![(0, 4)],
        vec![(0, 1)],
        vec![(2, 0)],
        vec![(0, 0), (0, 0)],
        vec![(0, usize::MAX - 3)],
    ] {
        plan.relocations = relocations;
        assert!(validate_plan(&plan).is_err());
    }
    plan.relocations.clear();
    plan.patches.push(patch);
    assert!(validate_plan(&plan)
        .unwrap_err()
        .to_string()
        .contains("Overlapping"));
}

#[test]
fn elf_parser_rejects_unresolved_text_and_invalid_section_ranges() -> Result<()> {
    let names = b"\0.text\0.shstrtab\0.rel.text\0";
    let names_offset = 52 + 4 * 40;
    let text_offset = names_offset + names.len();
    let mut blob = vec![0u8; text_offset + 4];
    blob[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    blob[18..20].copy_from_slice(&40u16.to_le_bytes());
    blob[32..36].copy_from_slice(&52u32.to_le_bytes());
    blob[46..48].copy_from_slice(&40u16.to_le_bytes());
    blob[48..50].copy_from_slice(&4u16.to_le_bytes());
    blob[50..52].copy_from_slice(&2u16.to_le_bytes());
    let section = |blob: &mut Vec<u8>, index: usize, fields: [u32; 10]| {
        for (field, value) in fields.into_iter().enumerate() {
            let start = 52 + index * 40 + field * 4;
            blob[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
    };
    section(
        &mut blob,
        1,
        [1, 1, 0, 0, text_offset as u32, 4, 0, 0, 4, 0],
    );
    section(
        &mut blob,
        2,
        [
            7,
            3,
            0,
            0,
            names_offset as u32,
            names.len() as u32,
            0,
            0,
            1,
            0,
        ],
    );
    blob[names_offset..text_offset].copy_from_slice(names);
    blob[text_offset..].copy_from_slice(&[0, 0, 0xa0, 0xe1]);
    assert_eq!(arm::elf_text(&blob)?.0, [0, 0, 0xa0, 0xe1]);
    for end in 0..blob.len() {
        assert!(arm::elf_text(&blob[..end]).is_err());
    }
    section(
        &mut blob,
        3,
        [17, 9, 0, 0, text_offset as u32, 8, 0, 1, 4, 8],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("Unresolved"));
    section(
        &mut blob,
        3,
        [17, 2, 0, 0, text_offset as u32, 4, 2, 0, 4, 0],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("symbol entry size"));
    Ok(())
}

#[test]
fn embedded_inputs_and_missing_tool() {
    let sources = source_hashes();
    assert!(sources.contains_key("runtime/installer.c"));
    assert!(sources.contains_key("crates/grkit-build/src/firmware.rs"));
    assert!(
        Toolchain::discover("/nonexistent/grkit-clang", "/nonexistent/grkit-lld")
            .unwrap_err()
            .to_string()
            .contains("Required tool not found")
    );
}

fn tools() -> Result<Toolchain> {
    Toolchain::discover(
        &std::env::var("GRKIT_CLANG").unwrap_or_else(|_| "clang".into()),
        &std::env::var("GRKIT_LLD").unwrap_or_else(|_| "ld.lld".into()),
    )
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_build_is_reproducible() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let body = synthetic_firmware();
    let first = build_image(&body, &directory.path().join("first"), &tools)?;
    let second = build_image(&body, &directory.path().join("second"), &tools)?;
    assert_eq!(first.files, second.files);
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.manifest["startup_activation"], "Off");
    assert_eq!(first.manifest["profile"], "production");
    assert!(first.manifest["source_revision"].is_null());
    assert!(first.manifest["source_dirty"].is_null());
    assert!(!serde_json::to_string(&first.manifest)?.contains(directory.path().to_str().unwrap()));
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(first.output.join("build.json"))?)?;
    assert_eq!(saved, first.manifest);
    let generated = fs::read(first.output.join("objects/module_payload.h"))?;
    assert_eq!(
        first.manifest["generated_header_sha256"],
        sha256(&generated)
    );
    let installer = &first.files["SCRIPT/PLUGINS/INSTALL.BIN"];
    assert_eq!(&installer[..6], b"\x7fELF\x01\x01");
    assert_eq!(&installer[16..20], &[2, 0, 40, 0]);
    for item in first.manifest["files"].as_array().unwrap() {
        let bytes = &first.files[item["path"].as_str().unwrap()];
        assert_eq!(item["size"], bytes.len());
        assert_eq!(item["sha256"], sha256(bytes));
    }
    assert!(build_image(&body, &first.output, &tools).is_err());
    for (path, bytes) in first.files {
        assert_eq!(fs::read(first.output.join("sd").join(path))?, bytes);
    }
    if let Ok(destination) = std::env::var("GRKIT_SYNTHETIC_OUTPUT") {
        let destination = Path::new(&destination);
        build_image(&body, destination, &tools)?;
        fs::write(destination.join("synthetic-body.bin"), &body.body)?;
    }
    Ok(())
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_hooks_match_fixed_target_contract() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let runtime = directory.path().join("runtime");
    materialize_runtime(&runtime)?;
    let body = synthetic_firmware();
    fs::write(runtime.join("hooks.h"), components::hooks_header(&body)?)?;
    let layout = &components::GR3_LAYOUT;
    let processing_region = layout.processing.unwrap();
    let framing_region = layout.framing.unwrap();
    let processing = components::processing(
        directory.path(),
        &runtime,
        &body,
        &tools,
        layout,
        processing_region,
    )?;
    let menu = components::menu(directory.path(), &runtime, &body, &tools, layout)?;
    let framing = components::framing(
        directory.path(),
        &runtime,
        &body,
        &tools,
        layout,
        framing_region,
    )?;

    // Reject changes to the widget's input and output context, not just its BL.
    for address in 0x531710bc..0x531710d8 {
        let mut changed = synthetic_firmware();
        changed.body[(address - target::RTOS_BASE) as usize] ^= 1;
        let error = components::processing(
            directory.path(),
            &runtime,
            &changed,
            &tools,
            layout,
            processing_region,
        )
        .err()
        .expect("Modified viewfinder instructions must be rejected");
        assert!(error
            .to_string()
            .contains("instructions changed at 0x531710bc"));
    }

    // Independent target contract: do not derive these expectations from component tables.
    let processing_hooks = [
        (0x536dc124, "module_parameters", "abfdffeb", true),
        (0x536dc28c, "module_processing", "940d08e3", false),
        (0x536dc68c, "module_packet", "903084e5", false),
        (0x536e8694, "module_gamma_flag", "9b30d4e5", false),
        (0x536e87c4, "module_gamma_curves", "891900eb", true),
    ];
    let menu_hooks = [
        (0x531a48f0, "menu_native_touch_0", "f431d0e5", false),
        (0x531a4904, "menu_native_touch_1", "f431d0e5", false),
        (0x531a4918, "menu_native_touch_2", "f431d0e5", false),
        (0x531a492c, "menu_native_touch_3", "f431d0e5", false),
        (0x531a4940, "menu_native_touch_4", "f431d0e5", false),
        (0x5319ff24, "menu_destroy", "0dc0a0e1", false),
        (0x531a4114, "menu_refresh", "0dc0a0e1", false),
        (0x531a4db8, "menu_event", "b030d1e1", false),
        (0x531a3e88, "menu_exit", "7022d0e5", false),
        (0x531a4cfc, "menu_changed", "0dc0a0e1", false),
        (0x533966fc, "menu_text_measure", "077193e7", false),
        (0x53397108, "menu_text_draw", "073193e7", false),
        (0x53209fb8, "menu_icon_width", "9e3200e3", false),
        (0x53209fec, "menu_icon_height", "9e3200e3", false),
        (0x5320a7cc, "menu_icon_guard", "9e1200e3", false),
        (0x5320a810, "menu_icon_descriptor", "033192e7", false),
        (0x531a0ba0, "menu_native_adj_apply", "0dc0a0e1", false),
        (0x531ab998, "menu_native_dialog_apply", "0dc0a0e1", false),
        (0x531ac270, "menu_native_page_event", "0dc0a0e1", false),
        (0x531ab1b0, "menu_native_page_exit", "0dc0a0e1", false),
        (0x531710c4, "menu_viewfinder_icon", "97e204eb", true),
        (0x53179b7c, "menu_playback_icon", "daf404eb", true),
        (0x532b7cfc, "menu_playback_detail", "eef1ffeb", true),
    ];
    let menu_instructions = [(0x535c251c, "0020a083", "01288083")];
    let framing_hooks = [
        (0x5317681c, "framing_update", "0dc0a0e1", false),
        (0x53176de4, "framing_exit", "0dc0a0e1", false),
        (0x53170b50, "framing_destroy", "0dc0a0e1", false),
        (0x53391b14, "framing_canvas", "0dc0a0e1", false),
        (0x53391be0, "framing_children", "500800eb", true),
        (0x535423f4, "framing_capture_request_hook", "0010d4e5", true),
        (0x536a2e48, "framing_height_0", "d33ea0e3", true),
        (0x536a2e34, "framing_height_1", "a93ea0e3", true),
        (0x536a2e20, "framing_height_2", "763ea0e3", true),
        (0x536a2e08, "framing_height_3", "383400e3", true),
        (0x536a2f98, "framing_screennail_inset", "2610a0e3", true),
        (0x536a3050, "framing_thumbnail_inset", "1010a0e3", true),
        (0x532b39b8, "framing_classify", "0dfcffeb", true),
        (0x532b3cac, "framing_image_aspect", "50fbffea", false),
        (0x536de1a8, "module_gamma_dirty", "0e30d4e5", false),
        (0x536e4774, "picture_post", "e9ffffeb", true),
        (0x535047d4, "picture_encode_entry", "0dc0a0e1", false),
        (0x53608080, "picture_capture_tone", "e1fdfeeb", true),
        (0x535031b0, "grain_particle", "9537d5e5", true),
        (0x5351361c, "grain_gate", "e7feffeb", true),
    ];
    let framing_instructions = [
        (0x53d553c8, "02010300", "00000300"),
        (0x534371d8, "1500008a", "1500001a"),
        (0x536a3228, "f83b0ae3", "080000ea"),
        (0x535ee8b4, "0c00000a", "0300000a"),
        (0x535ee8c0, "000052e3", "020052e1"),
    ];
    for (component, base, end, expected, instructions) in [
        (
            &processing,
            0x53002080,
            0x53002340u32,
            processing_hooks.as_slice(),
            [].as_slice(),
        ),
        (
            &menu,
            0x53002340,
            0x53004000u32,
            menu_hooks.as_slice(),
            menu_instructions.as_slice(),
        ),
        (
            &framing,
            0x540b6000,
            0x540be000u32,
            framing_hooks.as_slice(),
            framing_instructions.as_slice(),
        ),
    ] {
        assert_eq!(
            component.patches.len(),
            expected.len() + instructions.len() + 1
        );
        for (patch, &(address, before, after)) in component.patches[expected.len() + 1..]
            .iter()
            .zip(instructions)
        {
            assert_eq!(patch.address, address);
            assert_eq!(patch.before, hex::decode(before)?);
            assert_eq!(patch.after, hex::decode(after)?);
        }
        let code = &component.patches[0];
        assert_eq!(code.address, base);
        assert_eq!(code.after.len(), component.code_size);
        assert!(!code.after.is_empty());
        assert!(u64::from(base) + code.after.len() as u64 <= u64::from(end));
        assert_eq!(code.before, vec![0; code.after.len()]);

        let relocations: Vec<_> = code
            .after
            .chunks_exact(4)
            .enumerate()
            .filter_map(|(index, word)| (word == [0x40, 0x23, 0x01, 0x58]).then_some(index * 4))
            .collect();
        assert!(!relocations.is_empty());
        assert_eq!(component.relocations, relocations);

        for (patch, &(address, symbol, before, link)) in component.patches[1..].iter().zip(expected)
        {
            assert_eq!(patch.address, address, "{symbol}");
            assert_eq!(patch.before, hex::decode(before)?, "{symbol}");
            assert_eq!(patch.after.len(), 4, "{symbol}");
            let instruction = u32::from_le_bytes(patch.after.as_slice().try_into()?);
            assert_eq!(
                instruction >> 24,
                if link { 0xeb } else { 0xea },
                "{symbol}"
            );
            // A32 branches use a signed 24-bit word displacement from PC + 8.
            let displacement = ((instruction << 8) as i32 >> 6) as i64;
            let destination = i64::from(address) + 8 + displacement;
            let offset = component.symbols[symbol];
            assert!(offset < component.code_size as u32, "{symbol}");
            assert_eq!(offset % 4, 0, "{symbol}");
            assert_eq!(destination, i64::from(base) + i64::from(offset), "{symbol}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE and ARM toolchain"]
fn pinned_firmware_build_is_reproducible() -> Result<()> {
    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR3_210)?;
    let directory = tempfile::tempdir()?;
    let first = build_image(&body, &directory.path().join("first"), &tools)?;
    let second = build_image(&body, &directory.path().join("second"), &tools)?;
    assert_eq!(first.files, second.files);
    assert_eq!(first.manifest, second.manifest);
    Ok(())
}

// Fixed contracts captured from the retired implementation using synthetic data only.
// These checks deliberately do not derive their expected bytes from the Rust producer.
#[test]
fn generated_header_matches_fixed_synthetic_contract() -> Result<()> {
    let plan = PatchPlan {
        patches: vec![
            Patch {
                address: 0x53002080,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
            },
            Patch {
                address: 0x531a48f0,
                before: hex::decode("f431d0e5")?,
                after: hex::decode("000000ea")?,
            },
        ],
        relocations: vec![(0, 0)],
    };
    validate_plan(&plan)?;
    let header = loader::header(&plan, &synthetic_firmware())?;
    assert_eq!(
        sha256(header.as_bytes()),
        "0b69a53e4351f839cb0e393023421671164b8c07d1cf9e0967a8d7e8dc17d00a"
    );
    Ok(())
}

#[test]
fn generated_header_marks_only_rtos_tail_patches_as_unchecked() -> Result<()> {
    let plan = PatchPlan {
        patches: vec![
            Patch {
                address: 0x53002080,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
            },
            Patch {
                address: components::FRAMING_BASE,
                before: vec![0; 4],
                after: vec![1, 2, 3, 4],
            },
        ],
        relocations: vec![(0, 0)],
    };
    let header = loader::header(&plan, &synthetic_firmware())?;
    assert!(header.contains("{0x53002080,8,0,before_0,after_0},"));
    assert!(header.contains("{0x540b6000,4,1,before_1,after_1},"));
    Ok(())
}

#[test]
fn menu_assembly_symbols_are_typed_functions() {
    // Thumb menu C reaches these A32 symbols with BL. Without a function type,
    // the linker keeps BL instead of BLX and the call runs A32 code as Thumb.
    let source = include_str!("../../../runtime/menu.S");
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let mut globals = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = line.strip_prefix(".global ") else {
            continue;
        };
        globals += 1;
        assert_eq!(
            lines.get(index + 1).copied(),
            Some(format!(".type {name}, %function").as_str()),
            "{name}"
        );
    }
    assert!(globals > 0);
}

#[test]
fn production_scripts_and_activation_match_fixed_contracts() {
    let files = grkit_core::runtime::runtime_files(b"ELF");
    let expected = [
        (
            "SCRIPT/STARTUP.TTL",
            "bfa9f28e54f87af7a8191d915365c1617f533f1fdd2d6fcc47d0f549fe51083e",
        ),
        (
            "SCRIPT/PLUGINS/PLUGINS.TTL",
            "18873aba6432904392f3897932a740992d9859a7d8f3d233703b60174eb2dca1",
        ),
        (
            "SCRIPT/PLUGINS/INSTALL.BIN",
            "706abe3c90152075e656b661079730facf323f3ebccda7547ee1935c90845a09",
        ),
        (
            "SCRIPT/PLUGINS/SCAN.SH",
            "69529aeb6e5201c703cc0a2ef5d58ad48c0b473da84c75e242a74a0462ce8c86",
        ),
        (
            "SCRIPT/PLUGINS/INSTALL.SH",
            "eaa25cde3b7a957eae4de653150343abc556cbce826513fc5e70ed3bd95fe9af",
        ),
    ];
    assert_eq!(files.len(), expected.len());
    for (path, hash) in expected {
        assert_eq!(sha256(&files[path]), hash, "{path}");
    }
    assert!(!files
        .values()
        .any(|data| data.windows(8).any(|part| part == b"--select")));
    // Each camera gets the same files under its own ModeSet name.
    for (camera, modeset) in [
        (&grkit_core::camera::GR3, "00078350.588"),
        (&grkit_core::camera::GR3X, "00078490.609"),
        (&grkit_core::camera::GR4, "00078560.636"),
    ] {
        let activation = grkit_core::runtime::activation_files(camera);
        assert_eq!(activation.len(), 2);
        assert_eq!(
            sha256(&activation[modeset]),
            "94db9a80a0e5c8f6bd8f3fce41aff6b8758f58333193ed48eb0b6db7db76afb3"
        );
        assert_eq!(
            sha256(&activation["DEVELOP.MOD"]),
            "0d622b3b4e26b183a0434f876f33cf56787c9b37c9b05e850f4be4a15c8ccf2d"
        );
    }
}

#[test]
fn each_camera_offers_what_its_runtime_has() {
    for camera in grkit_core::camera::CAMERAS {
        let target = target::find(camera.target).unwrap();
        let layout = target.layout.unwrap();
        assert_eq!(layout.processing.is_some(), camera.effects, "{}", camera.id);
        // The installer hides the modules that the layout lacks (bit n + 1 of
        // the hidden mask is module name n).
        for (index, name) in grkit_core::modules::NAMES.iter().enumerate() {
            let absent = layout.hidden & (2 << index) != 0;
            assert_eq!(
                absent,
                !camera.modules.contains(name),
                "{} {name}",
                camera.id
            );
        }
    }
}
