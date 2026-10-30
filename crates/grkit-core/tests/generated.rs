//! Files generated from `grkit_core::abi`, `modules`, `camera`, and `site`. Set
//! GRKIT_UPDATE_GENERATED=1 to rewrite them after a change of the tables.

use std::{fs, path::Path};

#[test]
fn checked_in_generated_files_match_the_tables() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = vec![
        ("runtime/abi.h", grkit_core::abi::header()),
        ("runtime/native.h", grkit_core::native::header()),
    ];
    files.extend(grkit_core::modules::generated());
    files.extend(grkit_core::camera::generated());
    files.extend(grkit_core::site::generated());
    files.extend(grkit_core::recipes::generated());
    let update = std::env::var_os("GRKIT_UPDATE_GENERATED").is_some();
    let mut stale = Vec::new();
    for (name, expected) in files {
        let path = root.join(name);
        if update {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &expected).unwrap();
        } else if fs::read_to_string(&path).ok().as_deref() != Some(expected.as_str()) {
            stale.push(name);
        }
    }
    assert!(
        stale.is_empty(),
        "{stale:?} are stale. Run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated"
    );
}
