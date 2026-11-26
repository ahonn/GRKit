//! The boundary to the running camera: its control mailbox
//! (`runtime/control.h`). The camera answers each command in one poll of its
//! App task. The resources read the answers (`codec`, `camera_looks`).
use crate::protocol::Problem;
use std::time::Duration;

type Result<T> = std::result::Result<T, Problem>;

/// The App task polls every 100 ms. A camera that is busy with a capture
/// can answer later.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
/// The wait before a command that answered `CONTROL_CLOSING` goes again: ADJ
/// closes after the poll that answered.
const CLOSE_WAIT: Duration = Duration::from_millis(100);

pub trait RuntimeBackend: Send {
    /// The count of setting changes that the App task published, read from
    /// memory. None without a session with the camera: this starts none.
    fn state_changes(&mut self) -> Option<u32> {
        None
    }
    /// Mark the service as running or stopped for the camera, without a
    /// command.
    fn announce(&mut self, _running: bool) -> Result<()> {
        Err(unavailable())
    }
    /// A command of the mailbox (`grkit_core::abi`): its result code and
    /// response bytes. Only a mailbox that does not answer is an error; the
    /// resource of the command reads the result (`codec::Codes`).
    fn exchange(&mut self, _command: u32, _payload: &[u8]) -> Result<(u32, Vec<u8>)> {
        Err(unavailable())
    }
}

/// A service without a camera: the tests of the other resources, and the
/// build for another platform.
#[derive(Default)]
pub struct UnavailableBackend;
impl RuntimeBackend for UnavailableBackend {}

fn unavailable() -> Problem {
    Problem::new(
        503,
        "runtime_unavailable",
        "GRKit does not run, or its control mailbox was not found.",
    )
}
/// A page of GRKit shows on the camera (`control_idle`).
pub fn camera_busy() -> Problem {
    Problem::new(
        503,
        "camera_busy",
        "Close the GRKit page on the camera and try again.",
    )
}
pub(crate) fn invalid(detail: &'static str) -> Problem {
    Problem::new(503, "runtime_invalid", detail)
}

/// The mailbox of a camera that runs GRKit.
pub struct ControlBackend<M: crate::control::MailboxIo> {
    client: crate::control::Client<M>,
    nonce: [u8; 16],
}
pub type LinuxBackend = ControlBackend<crate::control::MappedMailbox>;

impl LinuxBackend {
    pub fn discover(root: &std::path::Path, nonce: [u8; 16]) -> anyhow::Result<Self> {
        anyhow::ensure!(nonce != [0; 16], "The HTTP boot nonce must be nonzero");
        Ok(Self::new(crate::control::discover(root)?, nonce))
    }
}
impl<M: crate::control::MailboxIo> ControlBackend<M> {
    pub fn new(client: crate::control::Client<M>, nonce: [u8; 16]) -> Self {
        Self { client, nonce }
    }
    fn raw(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
        use crate::control::Exchange;
        use grkit_core::abi::{CONTROL_BUSY, CONTROL_CLOSING};
        let deadline = std::time::Instant::now() + COMMAND_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self
                .client
                .exchange(self.nonce, command, payload, left)
                .map_err(|_| invalid("The control mailbox of the camera is not usable."))?
            {
                // The command did not apply, so it goes again once ADJ
                // closed. ADJ that stays open is a busy camera.
                Exchange::Answered(reply) if reply.result == CONTROL_CLOSING => {
                    if std::time::Instant::now() + CLOSE_WAIT >= deadline {
                        return Ok((CONTROL_BUSY, Vec::new()));
                    }
                    std::thread::sleep(CLOSE_WAIT);
                }
                answer => return Self::answer(answer),
            }
        }
    }
    fn answer(answer: crate::control::Exchange) -> Result<(u32, Vec<u8>)> {
        use crate::control::Exchange;
        match answer {
            Exchange::Answered(reply) => Ok((reply.result, reply.bytes)),
            Exchange::NotSent => Err(Problem::new(
                503,
                "runtime_busy",
                "The camera did not answer. Try again.",
            )),
            // The camera can still make the change: a second send of an add
            // or an undo would make it twice.
            Exchange::Unanswered => Err(Problem::new(
                503,
                "runtime_late",
                "The camera did not answer in time and can still make the change. Read it again before you try again.",
            )),
        }
    }
}
impl<M: crate::control::MailboxIo + Send> RuntimeBackend for ControlBackend<M> {
    fn state_changes(&mut self) -> Option<u32> {
        Some(self.client.state_changes())
    }
    fn announce(&mut self, running: bool) -> Result<()> {
        self.client
            .mark_service(running)
            .map_err(|_| invalid("The control mailbox of the camera is not usable."))
    }
    fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
        self.raw(command, payload)
    }
}

/// Discovery runs the installed core, which can fail while GRKit still
/// starts. The backend tries again later, and the other resources of the
/// service work meanwhile.
pub struct LazyLinuxBackend {
    root: std::path::PathBuf,
    nonce: [u8; 16],
    session: Option<LinuxBackend>,
    retry_after: Option<std::time::Instant>,
}
impl LazyLinuxBackend {
    pub fn new(root: std::path::PathBuf, nonce: [u8; 16]) -> Self {
        Self {
            root,
            nonce,
            session: None,
            retry_after: None,
        }
    }
    fn session(&mut self) -> Result<&mut LinuxBackend> {
        if self.session.is_none() {
            if self
                .retry_after
                .is_some_and(|deadline| std::time::Instant::now() < deadline)
            {
                return Err(unavailable());
            }
            match LinuxBackend::discover(&self.root, self.nonce) {
                Ok(session) => {
                    self.session = Some(session);
                    self.retry_after = None;
                }
                Err(_) => {
                    self.retry_after = Some(std::time::Instant::now() + Duration::from_secs(2));
                    return Err(unavailable());
                }
            }
        }
        Ok(self.session.as_mut().expect("discovered session"))
    }
}
impl RuntimeBackend for LazyLinuxBackend {
    fn state_changes(&mut self) -> Option<u32> {
        self.session.as_mut()?.state_changes()
    }
    /// A service that never found the mailbox has no mark to clear.
    fn announce(&mut self, running: bool) -> Result<()> {
        match (running, self.session.as_mut()) {
            (false, None) => Ok(()),
            (false, Some(session)) => session.announce(false),
            (true, _) => self.session()?.announce(true),
        }
    }
    fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
        self.session()?.exchange(command, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{tests::Memory, Client};
    use grkit_core::abi::{CONTROL_BUSY, CONTROL_LOOKS_READ, CONTROL_LOOK_TURN};

    #[test]
    fn a_command_gives_the_result_and_the_answer_of_the_camera() {
        let mut memory = Memory::new();
        memory.handler = Box::new(|_, payload| (CONTROL_BUSY, payload.to_vec()));
        let mut backend = ControlBackend::new(Client::new(memory).unwrap(), [7; 16]);
        // The result of a command is an answer, not an error.
        assert_eq!(
            backend.exchange(CONTROL_LOOK_TURN, &[1, 0, 0, 0]).unwrap(),
            (CONTROL_BUSY, vec![1, 0, 0, 0])
        );
        assert_eq!(
            UnavailableBackend
                .exchange(CONTROL_LOOKS_READ, &[])
                .unwrap_err()
                .code,
            "runtime_unavailable"
        );
    }

    #[test]
    fn a_change_that_closes_adj_goes_again_and_applies() {
        use grkit_core::abi::CONTROL_CLOSING;
        let mut memory = Memory::new();
        let mut closing = 1;
        memory.handler = Box::new(move |_, _| {
            if closing > 0 {
                closing -= 1;
                (CONTROL_CLOSING, Vec::new())
            } else {
                (0, vec![5, 0, 0, 0])
            }
        });
        let mut backend = ControlBackend::new(Client::new(memory).unwrap(), [7; 16]);
        assert_eq!(
            backend.exchange(CONTROL_LOOK_TURN, &[1, 0, 0, 0]).unwrap(),
            (0, vec![5, 0, 0, 0])
        );
        // ADJ that never closes ends as a busy camera, without a change.
        let mut memory = Memory::new();
        memory.handler = Box::new(|_, _| (CONTROL_CLOSING, Vec::new()));
        let mut backend = ControlBackend::new(Client::new(memory).unwrap(), [7; 16]);
        assert_eq!(
            backend
                .exchange(CONTROL_LOOK_TURN, &[1, 0, 0, 0])
                .unwrap()
                .0,
            CONTROL_BUSY
        );
    }
}
