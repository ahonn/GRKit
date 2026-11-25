//! An update of the package over Wi-Fi (`grkit_core::runtime::update`).
//!
//! The service checks the package (`grkit_core::release::verify_update`):
//! an official key signed its code, and the decoders check its data. It
//! checks that the release signs this camera and its firmware. Then it writes
//! the files that change to the update folder, reads each one back, and
//! writes the journal. The camera installs the update at the next power-on.
//!
//! The card has rules that the steps follow:
//! - It cannot rename a folder or replace a file, so the service never
//!   renames over a file, and the journal moves the files.
//! - After Linux deletes a file in a folder, a new name in that folder may not
//!   be found until the next power-on (GR IV). The service looks up each name
//!   that it creates in a folder before it deletes a file there, and it never
//!   deletes a folder.
//! - Linux keeps the result of a lookup of a name that does not exist, so it
//!   does not see a file that the camera creates under that name later. The
//!   service never looks up the journal by its name, which the camera
//!   creates; it reads the folder instead (`Card::journal_files`).
//!
//! - `card`: the card and its file operations.
//! - `record`: the software request and its record on the card.
//! - `summary`: what the card holds.
//! - `stage`: staging an update.

mod card;
mod record;
mod stage;
mod summary;
#[cfg(test)]
mod tests;

pub use card::{card_not_ready, Card, CardNotReady, JournalState};
pub(crate) use card::{look_up_unused, write_checked};
pub use record::{
    SoftwareContext, SoftwareIdentity, SoftwarePhase, SoftwareRequest, SoftwareStatus,
    REQUEST_REMOVED, SOFTWARE_REPLACED, STAGING_FAILED,
};
pub use stage::{stage, stage_request, Device, MAX_PACKAGE};
