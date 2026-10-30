//! The official modules: the module folders in `modules/` that each release
//! builds (`grkit module build`) and that the site offers.
//!
//! The default order of the modules in ADJ and in MENU is the `order` of
//! each manifest (`grkit_core::grm`), not this list.

use std::fmt::Write;

use crate::camera;

pub struct Official {
    /// The module ID (`grkit_core::grm`), which is also its file name.
    pub id: &'static str,
    /// The folder in `modules/`.
    pub folder: &'static str,
    /// The site key, in camel case.
    pub key: &'static str,
    /// The module shows only with effects (`CORE.TONE`), so the site adds the
    /// selected effects only with it.
    pub needs_effects: bool,
}

/// The official modules, in the order that the site shows them.
pub const OFFICIAL: [Official; 6] = [
    Official {
        id: "FILMLOOK",
        folder: "film-look",
        key: "filmLook",
        needs_effects: true,
    },
    Official {
        id: "FRAMING",
        folder: "framing",
        key: "framing",
        needs_effects: false,
    },
    Official {
        id: "GUIDES",
        folder: "guides",
        key: "guides",
        needs_effects: false,
    },
    Official {
        id: "DATE",
        folder: "date",
        key: "dateStamp",
        needs_effects: false,
    },
    Official {
        id: "GRAIN",
        folder: "grain",
        key: "grain",
        needs_effects: false,
    },
    Official {
        id: "RECIPES",
        folder: "recipes",
        key: "recipes",
        needs_effects: false,
    },
];

/// The official module of an ID.
pub fn official(id: &str) -> Option<&'static Official> {
    OFFICIAL.iter().find(|module| module.id == id)
}

/// `web/src/generated/modules.ts`: the official modules and the modules of
/// each camera.
fn site_module() -> String {
    let mut out = String::new();
    out.push_str(
        "// Generated from crates/grkit-core/src/modules.rs. Do not edit: change the\n\
         // list there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n",
    );
    out.push_str(
        "/** The official modules in menu order. `needsEffects`: the module shows only with effects. */\n",
    );
    out.push_str("export const OFFICIAL_MODULES = [\n");
    for module in &OFFICIAL {
        writeln!(
            out,
            "  {{ key: '{}', id: '{}', needsEffects: {} }},",
            module.key, module.id, module.needs_effects
        )
        .unwrap();
    }
    out.push_str("] as const;\n\n");
    out.push_str("export type ModuleKey = (typeof OFFICIAL_MODULES)[number]['key'];\n\n");
    out.push_str("/** The official modules that each camera can install, in menu order. */\n");
    out.push_str("export const CAMERA_MODULES = {\n");
    for camera in camera::CAMERAS {
        let keys: Vec<String> = OFFICIAL
            .iter()
            .filter(|module| camera.effects || !module.needs_effects)
            .map(|module| format!("'{}'", module.key))
            .collect();
        writeln!(out, "  {}: [{}],", camera.id, keys.join(", ")).unwrap();
    }
    out.push_str("} as const satisfies Readonly<Record<string, readonly ModuleKey[]>>;\n");
    out
}

/// The generated files, as paths from the repository root and contents.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![("web/src/generated/modules.ts", site_module())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_modules_have_folders_and_valid_ids() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules");
        for (i, module) in OFFICIAL.iter().enumerate() {
            assert!(crate::grm::valid_id(module.id));
            assert!(OFFICIAL[..i].iter().all(|other| other.id != module.id));
            let manifest = std::fs::read_to_string(root.join(module.folder).join("module.toml"))
                .unwrap_or_else(|_| panic!("modules/{} has no module.toml", module.folder));
            assert!(
                manifest.contains(&format!("id = \"{}\"", module.id)),
                "modules/{} is not {}",
                module.folder,
                module.id
            );
        }
        // Each module folder is official.
        let folders = std::fs::read_dir(&root).unwrap().count();
        assert_eq!(folders, OFFICIAL.len());
    }
}
