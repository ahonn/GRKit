use super::*;

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
    let mut body = synthetic_gr3();
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
        points: 0,
        before: vec![0; 4],
        after: target::HEADER_REFERENCE.to_le_bytes().to_vec(),
        site: None,
    };
    let mut plan = PatchPlan {
        exports: vec![],
        ready: 0,
        patches: vec![patch.clone()],
        relocations: vec![(0, 0)],
        references: Vec::new(),
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
    // An allocated, writable section (as .data or .bss would be).
    section(
        &mut blob,
        3,
        [17, 1, 3, 0, text_offset as u32, 4, 0, 0, 4, 0],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("not part of the private code"));
    Ok(())
}

#[test]
fn embedded_inputs_and_missing_tool() {
    let sources = source_hashes();
    assert!(sources.contains_key("runtime/installer.c"));
    assert!(sources.contains_key("crates/grkit-build/src/firmware.rs"));
    // Each source of grkit-core and its assets, also those that no list names.
    for path in [
        "crates/grkit-build/src/target/gr4_120.rs",
        "crates/grkit-core/src/grm.rs",
        "crates/grkit-core/src/hue.rs",
        "crates/grkit-core/src/fit/mod.rs",
        "crates/grkit-core/assets/RobotoCondensed-Light.ttf",
        "Cargo.lock",
    ] {
        assert!(sources.contains_key(path), "{path}");
    }
    assert!(!sources.keys().any(|path| path.contains("/src/tests/")));
    assert!(
        Toolchain::discover("/nonexistent/grkit-clang", "/nonexistent/grkit-lld")
            .unwrap_err()
            .to_string()
            .contains("Required tool not found")
    );
}
