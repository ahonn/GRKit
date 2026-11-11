use super::*;

mod builds;
mod effects;
mod inputs;
mod installer;
mod modules;
mod porting;
mod resolution;
mod targets;

/// Compares `text` with the snapshot `name` in `src/tests/snapshots`. With
/// GRKIT_UPDATE_GENERATED set, it writes the snapshot instead, so a change of
/// the output shows in the diff of the snapshot.
fn snapshot(name: &str, text: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/snapshots")
        .join(name);
    if std::env::var_os("GRKIT_UPDATE_GENERATED").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        expected == text,
        "The snapshot {name} changed. Review the change, then run \
         GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-build"
    );
}

/// `header` with each long array initializer as its number of values and the
/// SHA-256 of its text, so that a snapshot shows the other lines in full.
fn summarized(header: &str) -> String {
    header
        .lines()
        .map(|line| match line.split_once("= {") {
            Some((head, values)) if line.len() > 200 => {
                let values = values.trim_end_matches("};");
                format!(
                    "{head}= {{ /* {} values, SHA-256 {} */ }};\n",
                    values.split(',').count(),
                    sha256(values.as_bytes())
                )
            }
            _ => format!("{line}\n"),
        })
        .collect()
}

/// Where the synthetic image keeps its literal words, away from each site.
const SYNTHETIC_LITERALS: u32 = target::RTOS_BASE + 0x10000;

/// The words between the function starts of a synthetic image. The official
/// images have a function start about every 180 words. Without them, each
/// relocation window spans the largest function (`relocation::Planner`), and
/// the relocation tables are several times those of a real image.
const SYNTHETIC_FUNCTION_WORDS: usize = 192;

/// A synthetic RTOS image of `target`, for builds without the firmware. Its
/// words are pseudo-random, so each relocation window is unique as in a real
/// image, with a function start (`mov r12, sp`) every
/// SYNTHETIC_FUNCTION_WORDS words. Then it holds the stock bytes of the layout, the startup literals,
/// the Image Control tables and Standard record, and a literal word for each
/// address after `.data` that the target names, as firmware code that loads
/// such an address has.
fn synthetic_firmware(target: &'static target::Target) -> firmware::FirmwareImage {
    let size = (target.data_source - target::RTOS_BASE + target.data_size) as usize;
    // A fixed xorshift stream keeps the image the same on each host.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut body = Vec::with_capacity(size);
    while body.len() < size {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        body.extend_from_slice(&(state as u32).to_le_bytes());
    }
    for start in body.chunks_mut(SYNTHETIC_FUNCTION_WORDS * 4) {
        start[..4].copy_from_slice(&0xe1a0_c00du32.to_le_bytes());
    }
    let mut image = firmware::FirmwareImage {
        target,
        body,
        sha256: "synthetic".into(),
        decoded_sha256: "synthetic".into(),
    };
    let data = target.data_start..target.data_start + target.data_size;
    let mut write = |address: u32, bytes: &[u8]| {
        let address = if data.contains(&address) {
            address - target.data_start + target.data_source
        } else {
            address
        };
        let offset = (address - target::RTOS_BASE) as usize;
        image.body[offset..offset + bytes.len()].copy_from_slice(bytes);
    };
    let bss = target.data_start + target.data_size;
    let literals: BTreeSet<u32> = port::addresses(target)
        .unwrap()
        .iter()
        .filter(|address| address.kind != port::Kind::Site)
        .map(|address| address.address)
        .chain(target.relocation_bases.iter().map(|&(_, base)| base))
        .filter(|&address| address >= bss)
        .collect();
    for (index, address) in literals.iter().enumerate() {
        write(
            SYNTHETIC_LITERALS + index as u32 * 0x100,
            &address.to_le_bytes(),
        );
    }
    let layout = target.layout;
    for &(address, bytes) in layout.contexts.iter().chain(layout.guards) {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(_, address, bytes) in layout.sites {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, bytes, _) in layout
        .instruction_patches
        .iter()
        .chain(layout.framing_patches)
    {
        write(address, &hex::decode(bytes).unwrap());
    }
    let startup = [
        target.data_source,
        relocation::DATA_START,
        target.data_size,
        bss,
        target.heap_min,
    ];
    write(
        relocation::STARTUP_LITERALS,
        &startup
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let native = target.image_control.unwrap();
    for address in native.tables {
        write(
            address,
            &(0..native.table_bytes).map(|i| i as u8).collect::<Vec<_>>(),
        );
    }
    write(native.tables[0] + 11 * 4, &native.record.to_le_bytes());
    write(native.record, &(0..84u8).collect::<Vec<_>>());
    write(native.record + 4, &native.coefficients.to_le_bytes());
    for (index, &address) in literals.iter().enumerate() {
        let at = SYNTHETIC_LITERALS + index as u32 * 0x100;
        assert_eq!(image.read(at, 4).unwrap(), address.to_le_bytes(), "{at:#x}");
    }
    image
}

/// The synthetic image of GR III, which the fixed contracts use.
fn synthetic_gr3() -> firmware::FirmwareImage {
    synthetic_firmware(&target::GR3_210)
}

/// The targets that the synthetic ARM builds cover: one of each family.
const SYNTHETIC_TARGETS: [&target::Target; 2] = [&target::GR3_210, &target::GR4_111];

fn tools() -> Result<Toolchain> {
    Toolchain::discover(
        &std::env::var("GRKIT_CLANG").unwrap_or_else(|_| "clang".into()),
        &std::env::var("GRKIT_LLD").unwrap_or_else(|_| "ld.lld".into()),
    )
}

/// Resolves the relocation tables of `objects/module_payload.h` against an RTOS
/// image: the result line, then each symbol as (pinned, resolved).
fn resolve_image(output: &Path, image: &[u8]) -> Result<(String, Vec<(u32, u32)>)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rtos.bin");
    fs::write(&path, image)?;
    let executable = directory.path().join("relocation_image");
    let compiler = std::env::var("GRKIT_HOST_CC").unwrap_or_else(|_| "cc".into());
    let objects = output.join("objects");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = std::process::Command::new(compiler)
        .args(["-std=c11", "-O2", "-w", "-I"])
        .arg(objects.join("runtime"))
        .arg("-I")
        .arg(&objects)
        .arg(root.join("tests/runtime/relocation_image.c"))
        .arg("-o")
        .arg(&executable)
        .status()?;
    ensure!(status.success(), "Compile the relocation harness");
    let output = std::process::Command::new(&executable)
        .arg(&path)
        .output()?;
    ensure!(output.status.success(), "Run the relocation harness");
    let text = String::from_utf8(output.stdout)?;
    let mut lines = text.lines();
    let result = lines.next().context("No result")?.to_owned();
    let hex = |value: &str| u32::from_str_radix(value.trim_start_matches("0x"), 16);
    let symbols = lines
        .filter_map(|line| line.strip_prefix("symbol "))
        .map(|line| {
            let (pinned, resolved) = line.split_once(' ').context("Invalid symbol line")?;
            Ok((hex(pinned)?, hex(resolved)?))
        })
        .collect::<Result<_>>()?;
    Ok((result, symbols))
}
