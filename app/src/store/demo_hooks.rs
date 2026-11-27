//! What the demo screens (`dev::screens`) set directly, to show a state that
//! the demo camera alone does not reach, or holds still for a screenshot.
//! Debug builds and the `demo` feature only. The seeds of the Looks, recipes
//! and modules are next to their stores.

use std::time::Instant;

use grkit_core::release_index::IndexPreset;

use super::AppState;
use crate::domain::{
    CameraStatus, JournalState, PackageInfo, ReleaseProblem, UpdatePhase,
    release_notes::VersionNotes,
};

impl AppState {
    /// Choose `tab`, which shows once it is available.
    pub(crate) fn seed_tab(self, tab: super::Tab) {
        self.tab.set(tab);
    }

    pub(crate) fn seed_status(self, status: Option<CameraStatus>) {
        self.status.set(status);
    }

    pub(crate) fn seed_package(self, package: Option<PackageInfo>) {
        self.package.set(package);
    }

    /// The camera that the app connected to, as a connect picks it.
    pub(crate) fn seed_selected(self, peripheral_id: &str) {
        self.selected_id.set(Some(peripheral_id.to_owned()));
    }

    pub(crate) fn seed_release_notes(self, notes: Vec<VersionNotes>) {
        self.release_notes.set(notes);
    }

    pub(crate) fn seed_presets(self, presets: Vec<IndexPreset>) {
        self.recipes.presets.set(presets);
    }

    pub(crate) fn seed_release_check(self, checking: bool, problem: Option<ReleaseProblem>) {
        self.release_busy.set(checking);
        self.release_problem.set(problem);
    }

    pub(crate) fn seed_update(self, phase: UpdatePhase, baseline: Option<&str>) {
        self.update_shown.set(true);
        self.phase.set(phase);
        if let Some(baseline) = baseline {
            self.baseline.set(Some(baseline.to_owned()));
        }
    }

    pub(crate) fn seed_update_shown(self, shown: bool) {
        self.update_shown.set(shown);
    }

    pub(crate) fn seed_journal(self, journal: Option<JournalState>) {
        self.journal.set(journal);
    }

    /// A restart for changed Looks, or the connect that ends it.
    pub(crate) fn seed_look_restart(self, restarting: bool, connect: bool) {
        self.camera_restarting.set(restarting);
        self.restart_connect.set(connect);
        if connect {
            self.connect_started.set(Some(Instant::now()));
        }
    }

    pub(crate) fn seed_modules_editing(self) {
        self.modules.editing.set(true);
    }
}
