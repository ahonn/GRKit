//! The answers of the runtime to the mailbox commands of the recipes, the
//! modules, and the settings: little-endian words, a result code that each
//! command shares, and lists that come a page at a time.

use super::{problem, Result};
use crate::{
    protocol::Problem,
    runtime_backend::{camera_busy, RuntimeBackend},
};
use grkit_core::abi::{CONTROL_BUSY, CONTROL_CHANGED, CONTROL_INVALID, CONTROL_UNSUPPORTED};

/// The word at the byte `at` of `bytes`.
pub(super) fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The problems of the result codes of one resource.
pub(super) struct Codes {
    /// What the camera lists, as in "The recipe list of the camera".
    pub list: &'static str,
    /// The detail of `CONTROL_CHANGED`.
    pub changed: &'static str,
    /// The detail of `CONTROL_UNSUPPORTED`.
    pub unsupported: &'static str,
    /// The problem of `CONTROL_INVALID`, when the resource has one.
    pub invalid: Option<fn() -> Problem>,
}

impl Codes {
    /// An answer that does not have the form of the command.
    pub(super) fn malformed(&self) -> Problem {
        Problem::new(
            503,
            "runtime_invalid",
            format!("{} of the camera is malformed.", self.list),
        )
    }

    /// The answer `bytes` of a command whose result is `result`.
    pub(super) fn checked(&self, (result, bytes): (u32, Vec<u8>)) -> Result<Vec<u8>> {
        match result {
            0 => Ok(bytes),
            CONTROL_BUSY => Err(camera_busy()),
            CONTROL_CHANGED => Err(problem(409, "runtime_changed", self.changed)),
            CONTROL_INVALID => Err(self.invalid.map_or_else(rejected, |invalid| invalid())),
            CONTROL_UNSUPPORTED => Err(problem(
                503,
                "runtime_protocol_unsupported",
                self.unsupported,
            )),
            _ => Err(rejected()),
        }
    }

    /// Reads a list that the command `command` answers a page at a time,
    /// from the row of its payload. `page` reads one answer: it keeps its
    /// entries and gives the number of rows, the first row of the page, and
    /// the number of its entries. Returns the last answer, for the words of
    /// its header.
    pub(super) fn read_pages(
        &self,
        backend: &mut dyn RuntimeBackend,
        command: u32,
        mut page: impl FnMut(&[u8]) -> Result<Page>,
    ) -> Result<Vec<u8>> {
        let mut first = 0u32;
        loop {
            let bytes = self.checked(backend.exchange(command, &first.to_le_bytes())?)?;
            let Page { count, at, entries } = page(&bytes)?;
            if at != first {
                return Err(self.malformed());
            }
            first += entries;
            if first >= count || entries == 0 {
                if first != count {
                    return Err(self.malformed());
                }
                return Ok(bytes);
            }
        }
    }
}

fn rejected() -> Problem {
    problem(503, "runtime_invalid", "The camera rejected the command.")
}

/// What a page of a list gives.
pub(super) struct Page {
    /// The number of rows of the list.
    pub count: u32,
    /// The first row of the page.
    pub at: u32,
    /// The number of entries of the page.
    pub entries: u32,
}
