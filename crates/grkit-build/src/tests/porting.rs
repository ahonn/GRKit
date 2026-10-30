use super::*;

#[test]
fn port_lists_the_addresses_of_each_target() -> Result<()> {
    for target in target::TARGETS {
        let addresses = port::addresses(target)?;
        let names: BTreeSet<_> = addresses.iter().map(|a| (a.kind, a.name.clone())).collect();
        assert_eq!(names.len(), addresses.len(), "{}", target.name);
        // Each hook of the layout is a site, and each header address is listed.
        assert_eq!(
            addresses
                .iter()
                .filter(|a| a.kind == port::Kind::Site && a.symbol.is_some())
                .count(),
            components::hooks(target)?.len(),
            "{}",
            target.name
        );
        let header = std::str::from_utf8(runtime_source(&target.header()).unwrap())?;
        assert_eq!(
            addresses
                .iter()
                .filter(|a| a.kind == port::Kind::Header)
                .count(),
            header.matches("FIRMWARE(0x").count(),
            "{}",
            target.name
        );
    }
    Ok(())
}

/// Rust code without comments and whitespace, and without the trailing comma
/// of a list, so that comments and formatting do not count.
fn rust_items(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap())
        .collect::<String>()
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")")
        .replace(",]", "]")
        .replace(",}", "}")
}

/// `grkit port` drafts the module of the new build. A target ported to its own
/// image gives the items of its checked-in module, so a draft needs only the
/// review of its `PORT` comments and of the values that are not addresses.
#[test]
fn port_drafts_the_module_of_each_target() -> Result<()> {
    for target in target::TARGETS {
        let layout = target.layout;
        let addresses = port::addresses(target)?
            .into_iter()
            .map(|address| port::Ported {
                port: Ok(address.address),
                sources: 1,
                review: None,
                stock: address.stock.clone(),
                address,
            })
            .collect();
        let port = port::Port {
            base: target,
            radius: port::RADIUS,
            layout: relocation::ImageLayout {
                data_source: target.data_source,
                data_size: target.data_size,
                heap_min: target.heap_min,
                tail: layout.tail.0,
            },
            addresses,
            relocatable: true,
            identity: None,
        };
        let module = format!("crates/grkit-build/src/target/{}.rs", target.module());
        let (_, source) = crate::BUILD_SOURCES
            .iter()
            .find(|(name, _)| *name == module)
            .unwrap_or_else(|| panic!("BUILD_SOURCES has no {module}"));
        assert_eq!(
            rust_items(&port.draft_target()?),
            rust_items(std::str::from_utf8(source)?),
            "{module}"
        );
    }
    Ok(())
}

/// GR IV 1.11 ported to 1.20 gives no address that differs from the gr4-1.20
/// target without a review. The image of each build is enough; no toolchain.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4 and GRKIT_TEST_FIRMWARE_GR4_120"]
fn port_of_gr4_111_to_120_agrees_with_the_gr4_120_target() -> Result<()> {
    let path = |name: &str| -> Result<PathBuf> {
        Ok(PathBuf::from(
            std::env::var(name).with_context(|| format!("Set {name}"))?,
        ))
    };
    let base = verify_firmware(&path("GRKIT_TEST_FIRMWARE_GR4")?, &target::GR4_111)?;
    let other = verify_firmware(&path("GRKIT_TEST_FIRMWARE_GR4_120")?, &target::GR4_120)?;
    let port = port::port(&base, &other.body, port::RADIUS)?;
    let check = port::check(&port, &target::GR4_120)?;
    assert_eq!(check["differ"], json!([]), "{check:#}");
    // The type tables grew in 1.20, so their starts need a review.
    let review: Vec<_> = check["review"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(
        review.contains(&"target:image_control.tables[0]"),
        "{check:#}"
    );
    // 1.20 has one more parameter setter in the run of equal setters, so the
    // windows of the later setters match one setter too far. Shading and clarity
    // need a review.
    for row in [7, 8] {
        let name = format!("site:hook:menu_parameter_set_{row}");
        assert!(review.contains(&name.as_str()), "{check:#}");
    }
    assert!(check["equal"].as_u64().unwrap() >= 130, "{check:#}");
    assert_eq!(port.layout.data_source, target::GR4_120.data_source);
    assert_eq!(port.layout.data_size, target::GR4_120.data_size);
    assert_eq!(port.layout.heap_min, target::GR4_120.heap_min);
    Ok(())
}
