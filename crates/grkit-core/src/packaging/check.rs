//! The checks of the data files of a package: effects, modules, imports,
//! recipes, and the guide.

use super::*;

/// The checks of the package files that are data or modules, which a package
/// built on a phone or a page can hold: effects, modules, the recipe and
/// import libraries, and the package list. The runtime has its own checks.
pub(crate) struct DataCheck {
    camera: &'static Camera,
    ids: BTreeSet<String>,
    module_ids: BTreeSet<String>,
    native_slots: BTreeMap<u32, String>,
    status_icons: usize,
    listed: Vec<String>,
    imports: usize,
    recipes: usize,
}

/// What `DataCheck` found.
pub(crate) struct Data {
    pub effect_ids: Vec<String>,
    pub modules: Vec<String>,
    pub imports: usize,
    pub recipes: usize,
}

impl DataCheck {
    pub(crate) fn new(camera: &'static Camera) -> Self {
        DataCheck {
            camera,
            ids: BTreeSet::new(),
            module_ids: BTreeSet::new(),
            native_slots: BTreeMap::new(),
            status_icons: 0,
            listed: Vec::new(),
            imports: 0,
            recipes: 0,
        }
    }

    /// Checks the package entry `name`: true for a file of the data, false
    /// for another file.
    pub(crate) fn check(&mut self, name: &str, blob: &[u8]) -> Result<bool> {
        let camera = self.camera;
        ensure!(
            !name.starts_with(&format!("SD_ROOT/{}/", crate::card::DATA_FOLDER_PATH)),
            "A package holds no file of the data folder: {name}"
        );
        if name == format!("SD_ROOT/{RECIPES_PACKAGE_PATH}") {
            let library = recipes::decode(blob).context("Read the recipe library")?;
            ensure!(
                library.iter().all(|recipe| camera
                    .targets()
                    .iter()
                    .all(|target| target.starts_with(recipe.family.native().key()))),
                "The recipe library has a recipe of another camera"
            );
            self.recipes = library.len();
        } else if is_effect_path(name) {
            let id = effects::decode(blob)?.0.id;
            ensure!(
                name.rsplit('/').next() == Some(effects::filename(&id)?.as_str()),
                "Profile filename does not match its stable ID"
            );
            ensure!(self.ids.insert(id), "Duplicate effect ID");
            self.listed
                .push(name.rsplit('/').next().unwrap_or_default().to_owned());
        } else if let Some(file) = name
            .strip_prefix("SD_ROOT/")
            .and_then(|path| path.strip_prefix(crate::grm::FOLDER))
            .and_then(|path| path.strip_prefix('/'))
        {
            let module = crate::grm::Module::decode(blob)
                .with_context(|| format!("Read the module {file}"))?;
            ensure!(
                file == format!("{}.GRM", module.id),
                "The module file {file} is not named by its ID {}",
                module.id
            );
            for placement in module
                .indicator_placements()
                .with_context(|| format!("Read the viewfinder indicators of {}", module.id))?
            {
                match placement {
                    crate::grm::IndicatorPlacement::NativeSlot(slot) => {
                        if let Some(owner) = self.native_slots.insert(slot, module.id.clone()) {
                            bail!("Viewfinder slot image_control is registered by both {owner} and {}", module.id);
                        }
                    }
                    crate::grm::IndicatorPlacement::StatusRow => {
                        self.status_icons += 1;
                        ensure!(
                            self.status_icons <= crate::abi::STATUS_ICONS_MAX,
                            "The package exceeds the viewfinder status-row capacity of {} icons at module {}",
                            crate::abi::STATUS_ICONS_MAX,
                            module.id
                        );
                    }
                }
            }
            self.listed.push(file.to_owned());
            self.module_ids.insert(module.id);
        } else if name == format!("SD_ROOT/{}", card::IMPORTS_FILE) {
            self.imports = imports::decode(blob)
                .context("Read the import library")?
                .len();
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// Checks the package list `list` against the files, and the capacity.
    pub(crate) fn finish(self, list: &[u8]) -> Result<Data> {
        let DataCheck {
            camera,
            ids,
            module_ids,
            listed,
            imports,
            recipes,
            ..
        } = self;
        // The effects that the list leaves out are effects that the package does not hold.
        let left_out: BTreeSet<String> = std::str::from_utf8(list)
            .context("Read the package list")?
            .lines()
            .filter_map(|line| line.strip_prefix('-'))
            .map(str::to_owned)
            .collect();
        ensure!(
            left_out
                .iter()
                .all(|name| effect_file_name(name) && !listed.contains(name)),
            "{PACKAGE_LIST} leaves out an effect that is not valid or that the package holds"
        );
        ensure!(
            *list == package_list(listed.iter(), &left_out),
            "{PACKAGE_LIST} must list the effect and module files of the package"
        );
        ensure!(
            ids.len() <= runtime::MAX_COUNT,
            "Target effect capacity exceeded"
        );
        ensure!(
            camera.effects || ids.is_empty(),
            "{} has no Image Control yet, so a package for it cannot have effects",
            camera.name
        );
        // The effects show only on the page of Image Control, which shows only
        // with effects.
        let needs_effects =
            |id: &String| modules::official(id).is_some_and(|module| module.needs_effects);
        ensure!(
            ids.is_empty() || module_ids.iter().any(needs_effects),
            "The effects need the Image Control module"
        );
        ensure!(
            module_ids
                .iter()
                .any(|id| !needs_effects(id) || !ids.is_empty()),
            "The package shows nothing: add a module other than Image Control, or add effects"
        );
        Ok(Data {
            effect_ids: ids.into_iter().collect(),
            modules: module_ids.into_iter().collect(),
            imports,
            recipes,
        })
    }
}

/// Checks that `pdf` is a PDF document. The release builds the guides.
pub(super) fn check_guide(pdf: &[u8]) -> Result<()> {
    ensure!(
        pdf.starts_with(b"%PDF-"),
        "The installation guide is not a PDF document"
    );
    Ok(())
}
