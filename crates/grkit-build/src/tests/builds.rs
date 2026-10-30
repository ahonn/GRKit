use super::*;

/// The synthetic builds relocate as the production builds do: the pinned
/// image resolves each symbol of the installer to its pinned address.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_build_is_reproducible() -> Result<()> {
    let tools = tools()?;
    for target in SYNTHETIC_TARGETS {
        let directory = tempfile::tempdir()?;
        let body = synthetic_firmware(target);
        let first = build_image(&body, &directory.path().join("first"), &tools)?;
        let second = build_image(&body, &directory.path().join("second"), &tools)?;
        assert_eq!(first.files, second.files);
        assert_eq!(first.manifest, second.manifest);
        assert_eq!(first.manifest["startup_activation"], "Off");
        assert_eq!(first.manifest["profile"], "production");
        assert_eq!(first.manifest["version"], env!("CARGO_PKG_VERSION"));
        assert!(first.manifest["source_revision"].is_null());
        assert!(first.manifest["source_dirty"].is_null());
        assert!(
            !serde_json::to_string(&first.manifest)?.contains(directory.path().to_str().unwrap())
        );
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(first.output.join("build.json"))?)?;
        assert_eq!(saved, first.manifest);
        let generated = fs::read(first.output.join("objects/module_payload.h"))?;
        assert_eq!(
            first.manifest["generated_header_sha256"],
            sha256(&generated)
        );
        let installer = &first.files["SCRIPT/GRKIT/CORE.BIN"];
        assert_eq!(&installer[..6], b"\x7fELF\x01\x01");
        assert_eq!(&installer[16..20], &[2, 0, 40, 0]);
        for item in first.manifest["files"].as_array().unwrap() {
            let bytes = &first.files[item["path"].as_str().unwrap()];
            assert_eq!(item["size"], bytes.len());
            assert_eq!(item["sha256"], sha256(bytes));
        }
        assert!(build_image(&body, &first.output, &tools).is_err());
        for (path, bytes) in &first.files {
            assert_eq!(&fs::read(first.output.join("sd").join(path))?, bytes);
        }
        let (result, symbols) = resolve_image(&first.output, &body.body)?;
        let tail = target.layout.tail.0;
        assert_eq!(result, format!("ok {tail:#x}"), "{}", target.name);
        assert!(!symbols.is_empty());
        assert!(
            symbols.iter().all(|(pinned, resolved)| pinned == resolved),
            "{}",
            target.name
        );
        if let Ok(destination) = std::env::var("GRKIT_SYNTHETIC_OUTPUT") {
            let destination = Path::new(&destination).join(target.name);
            build_image(&body, &destination, &tools)?;
            fs::write(destination.join("synthetic-body.bin"), &body.body)?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_hooks_match_the_target_tables() -> Result<()> {
    let tools = tools()?;
    // The regions of GR III, independent of the layout methods.
    let gr3 = target::GR3_210.layout;
    assert_eq!(gr3.processing(), (0x540b6100, 0x540b6500));
    assert_eq!(gr3.menu(), (0x540b6500, 0x540c6000));
    // Framing starts after the menu code, at the next 256-byte boundary.
    assert_eq!(gr3.framing(0x540bb3f4), (0x540bb400, 0x540c6000));
    assert_eq!(gr3.framing(0x540bb400), (0x540bb400, 0x540c6000));
    assert_eq!(gr3.framing(0), (0x540b6500, 0x540c6000));
    for target in SYNTHETIC_TARGETS {
        synthetic_hooks(target, &tools)?;
    }
    Ok(())
}

fn synthetic_hooks(target: &'static target::Target, tools: &Toolchain) -> Result<()> {
    let directory = tempfile::tempdir()?;
    let runtime = directory.path().join("runtime");
    materialize_runtime(&runtime)?;
    let body = synthetic_firmware(target);
    fs::write(runtime.join("hooks.h"), components::hooks_header(&body)?)?;
    let layout = target.layout;
    let processing = components::processing(directory.path(), &runtime, &body, tools)?;
    let menu = components::menu(directory.path(), &runtime, &body, tools)?;
    let menu_end = menu.patches[0].address + menu.code_size as u32;
    let framing = components::framing(directory.path(), &runtime, &body, tools, menu_end)?;

    // Reject changes to the widget's input and output context, not just its BL.
    let &(widget, stock) = layout.contexts.last().unwrap();
    for address in widget..widget + stock.len() as u32 / 2 {
        let mut changed = synthetic_firmware(target);
        changed.body[(address - target::RTOS_BASE) as usize] ^= 1;
        let error = components::processing(directory.path(), &runtime, &changed, tools)
            .err()
            .expect("Modified viewfinder instructions must be rejected");
        assert!(error
            .to_string()
            .contains(&format!("instructions changed at {widget:#x}")));
    }

    // Each component installs the hooks of its part in family order, then its
    // in-place changes. Each patch moves with its firmware site.
    let hooks = components::hooks(body.target)?;
    for (component, part, (base, end), instructions) in [
        (
            &processing,
            components::Part::Processing,
            layout.processing(),
            [].as_slice(),
        ),
        (
            &menu,
            components::Part::Menu,
            layout.menu(),
            layout.instruction_patches,
        ),
        (
            &framing,
            components::Part::Framing,
            layout.framing(menu_end),
            layout.framing_patches,
        ),
    ] {
        let expected: Vec<_> = hooks.iter().filter(|hook| hook.spec.part == part).collect();
        assert!(!expected.is_empty());
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
            assert_eq!(patch.site, Some(Anchor::Firmware(address)));
        }
        assert!(!component.references.is_empty());
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

        for (patch, hook) in component.patches[1..].iter().zip(expected) {
            let symbol = hook.spec.symbol;
            assert_eq!(patch.address, hook.address, "{symbol}");
            assert_eq!(patch.site, Some(Anchor::Firmware(hook.address)), "{symbol}");
            assert_eq!(patch.before, hex::decode(hook.stock)?, "{symbol}");
            assert_eq!(patch.points, hook.spec.point.bit(), "{symbol}");
            assert_eq!(patch.after.len(), 4, "{symbol}");
            let instruction = u32::from_le_bytes(patch.after.as_slice().try_into()?);
            assert_eq!(
                instruction >> 24,
                if hook.spec.link { 0xeb } else { 0xea },
                "{symbol}"
            );
            // A32 branches use a signed 24-bit word displacement from PC + 8.
            let displacement = ((instruction << 8) as i32 >> 6) as i64;
            let destination = i64::from(hook.address) + 8 + displacement;
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

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn dump_tool_is_a_static_arm_linux_program() -> Result<()> {
    let files = dump::files(&tools()?)?;
    let names: Vec<_> = files.keys().map(String::as_str).collect();
    let mut expected = grkit_core::packaging::DUMP_FILES.to_vec();
    expected.sort();
    assert_eq!(names, expected);
    let binary = &files["SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN"];
    let half = |offset: usize| u16::from_le_bytes([binary[offset], binary[offset + 1]]);
    let word = |offset: usize| u32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap());
    // ELF32, little endian, an executable for ARM.
    assert_eq!(&binary[..6], b"\x7fELF\x01\x01");
    assert_eq!((half(16), half(18)), (2, 40));
    // No program header asks for an interpreter, so the camera needs no libc.
    let (table, size, count) = (word(28) as usize, half(42) as usize, half(44) as usize);
    assert!((0..count).all(|index| word(table + index * size) != 3));
    assert!(binary.windows(8).any(|part| part == b"/dev/mem"));
    // The script runs the program from the folder where the program writes.
    let script = std::str::from_utf8(&files["SD_ROOT/SCRIPT/GRDUMP/DUMP.SH"])?;
    assert!(script.starts_with("/media/storage1/SCRIPT/GRDUMP/DUMP.BIN\n"));
    Ok(())
}
