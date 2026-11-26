//! The resources of the running camera and its card: the Looks (`looks`),
//! the recipes (`recipes`), the module settings (`modules`), and the GRKit
//! settings and the shutdown picture (`settings`). The service routes each
//! request to its handler with its typed route and admits each change
//! before (`api::Service`). The camera answers the commands of each
//! handler in its mailbox (`runtime_backend`, `crate::codec`).

use crate::{protocol::Problem, runtime_backend::RuntimeBackend};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

mod looks;
mod modules;
mod recipes;
mod settings;

pub use looks::MAX_LOADED;

type Result<T> = std::result::Result<T, Problem>;

/// What the handlers of a request read.
pub struct RequestContext<'a> {
    pub boot_id: &'a str,
    /// The identity of the card, when the service has it.
    pub card_id: Option<String>,
    /// The mounted card.
    pub root: &'a Path,
    /// The keys that sign the release whose official Looks the card names.
    pub release_keys: &'a [[u8; 32]],
    /// The recipe family of the camera, when its model is known.
    pub family: Option<grkit_core::recipes::Family>,
}

/// The resources of the camera and the state that the service keeps of
/// them while it runs.
pub struct Resources {
    backend: Mutex<Box<dyn RuntimeBackend>>,
    session: Mutex<Option<looks::Session>>,
    modules: Mutex<modules::ModuleCache>,
}

impl Resources {
    pub fn new(backend: Box<dyn RuntimeBackend>) -> Self {
        Self {
            backend: Mutex::new(backend),
            session: Mutex::new(None),
            modules: Mutex::default(),
        }
    }

    /// The count of setting changes of the running camera, when the service
    /// has a session with it and no request holds it. Sends no command.
    pub fn state_changes(&self) -> Option<u32> {
        crate::sync::try_lock(&self.backend)?.state_changes()
    }

    /// Mark the service as running or stopped for the camera. True when the
    /// mark is in place.
    pub fn announce(&self, running: bool) -> bool {
        lock(&self.backend).announce(running).is_ok()
    }
}

use crate::sync::lock;

/// Takes `mutex`, or answers that the camera answers another request.
fn try_lock<T: ?Sized>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    crate::sync::try_lock(mutex).ok_or_else(|| {
        problem(
            503,
            "runtime_busy",
            "The camera is answering another request.",
        )
    })
}

/// Write the whole file in place and read it back (`update::write_checked`).
/// True when the card holds `bytes`.
fn write_checked(path: &Path, bytes: &[u8]) -> bool {
    crate::update::write_checked(path, bytes).is_ok()
}

fn problem(status: u16, code: &'static str, detail: &'static str) -> Problem {
    Problem::new(status, code, detail)
}
