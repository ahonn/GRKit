//! The boundary to the running camera: the choices of its effects (runtime/control_looks.h). The camera answers each
//! command in one poll of its App task. No command reads the card, loads an
//! effect, or removes one: effect files apply at the next start.
use crate::protocol::Problem;
use grkit_core::abi::*;
use std::time::Duration;

type Result<T> = std::result::Result<T, Problem>;

/// The App task polls every 100 ms. A camera that is busy with a capture
/// can answer later.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

/// One effect that the running camera lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLook {
    /// The state ID of `CORE.EFFECT.<ID>` (`look_key`).
    pub key: u32,
    pub enabled: bool,
    /// The position that MENU gave the effect, from 1, or 0 without one.
    pub position: u32,
    /// The effect has a record: it applies at once when it is On.
    pub loaded: bool,
}

/// The effect list of the running camera, in the order of this start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeLooks {
    pub looks: Vec<RuntimeLook>,
    /// The key of the committed effect.
    pub selected: Option<u32>,
    /// The count of setting changes of the camera (`HOST_STATE_CHANGES`).
    pub state_changes: u32,
    /// Each recipe or mode-dial set that names an effect.
    pub uses: Vec<LookUse>,
}

/// A recipe or a mode-dial set that names an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookUse {
    /// The recipe code of the effect.
    pub code: u32,
    pub origin: UseOrigin,
}

/// Where a use is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UseOrigin {
    /// The recipe in `slot`: the name that the person gave it, when the
    /// camera sent it (None for the automatic name), and its number.
    Recipe {
        slot: u32,
        number: u32,
        name: Option<String>,
    },
    /// The set of user mode `mode` of the mode dial (U1 is 1).
    Dial { mode: u32 },
    /// A camera that does not say.
    Unknown,
}

impl LookUse {
    /// The use of `word`, and its recipe name from `name` when it has one.
    fn decode(word: u32, name: Option<&[u32]>) -> Self {
        let origin = word >> CONTROL_USE_ORIGIN_SHIFT & CONTROL_USE_ORIGIN_MASK;
        let origin = match origin {
            0 => UseOrigin::Unknown,
            mode if word & CONTROL_USE_DIAL != 0 => UseOrigin::Dial { mode },
            slot => UseOrigin::Recipe {
                slot: slot - 1,
                number: word >> CONTROL_USE_NUMBER_SHIFT & CONTROL_USE_NUMBER_MASK,
                name: name.and_then(recipe_name),
            },
        };
        LookUse {
            code: word & 0x7fff,
            origin,
        }
    }
}

/// A recipe name of four 7-bit ASCII characters in each word, the first in
/// the low bits, up to the first 0. None for the automatic name or a name
/// that is not printable ASCII.
fn recipe_name(words: &[u32]) -> Option<String> {
    if words.iter().any(|word| word >> 28 != 0) {
        return None;
    }
    let name: Vec<u8> = words
        .iter()
        .flat_map(|word| (0..4).map(move |i| (word >> (7 * i) & 0x7f) as u8))
        .take_while(|c| *c != 0)
        .collect();
    let printable = name.iter().all(|c| (0x20..=0x7e).contains(c));
    (printable && !name.is_empty()).then(|| String::from_utf8_lossy(&name).into_owned())
}

impl RuntimeLooks {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let malformed = || invalid("The effect list of the camera is malformed.");
        let (words, rest) = bytes.as_chunks::<4>();
        let words: Vec<u32> = words.iter().map(|word| u32::from_le_bytes(*word)).collect();
        let header = CONTROL_LOOKS_HEADER / 4;
        let entry = CONTROL_LOOK_ENTRY / 4;
        if !rest.is_empty() || words.len() < header {
            return Err(malformed());
        }
        let (count, codes) = (words[0] as usize, words[3] as usize);
        if count > crate::looks::MAX_LISTED || words.len() < header + count * entry + codes {
            return Err(malformed());
        }
        let looks: Vec<_> = words[header..header + count * entry]
            .chunks(entry)
            .map(|look| RuntimeLook {
                key: look[0],
                enabled: look[1] & 1 == 0,
                position: look[1] >> 1,
                loaded: look[2] != 0,
            })
            .collect();
        let mut keys: Vec<_> = looks.iter().map(|look| look.key).collect();
        keys.sort_unstable();
        keys.dedup();
        if keys.len() != looks.len() || keys.first() == Some(&0) {
            return Err(malformed());
        }
        Ok(Self {
            looks,
            selected: (words[1] != 0).then_some(words[1]),
            state_changes: words[2],
            uses: decode_uses(&words[header + count * entry..], codes).ok_or_else(malformed)?,
        })
    }

    /// The keys in the order that MENU shows: an effect with a position
    /// first, then the others in the order of this start (runtime/effects.h).
    pub fn menu_order(&self) -> Vec<u32> {
        let mut order: Vec<_> = self.looks.iter().enumerate().collect();
        order.sort_by_key(|(index, look)| match look.position {
            0 => (1, *index as u32),
            position => (0, position),
        });
        order.into_iter().map(|(_, look)| look.key).collect()
    }
}

/// The `count` uses after the effects: each word, then the name words of a
/// named recipe. None unless the words end with the last use.
fn decode_uses(words: &[u32], count: usize) -> Option<Vec<LookUse>> {
    let mut uses = Vec::with_capacity(count.min(words.len()));
    let mut at = 0;
    for _ in 0..count {
        let word = *words.get(at)?;
        at += 1;
        let name = if word & CONTROL_USE_NAMED != 0 {
            let name = words.get(at..at + RECIPE_NAME_WORDS)?;
            at += RECIPE_NAME_WORDS;
            Some(name)
        } else {
            None
        };
        uses.push(LookUse::decode(word, name));
    }
    (at == words.len()).then_some(uses)
}

/// The key of an effect in the running camera: the state ID of its choice.
pub fn look_key(id: &str) -> u32 {
    grkit_core::runtime::state_key_id(&format!("CORE.EFFECT.{id}"))
}

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
    fn looks(&mut self) -> Result<RuntimeLooks> {
        Err(unavailable())
    }
    /// Turn an effect on or off, as its checkbox in MENU does.
    fn turn(&mut self, _key: u32, _off: bool) -> Result<()> {
        Err(unavailable())
    }
    /// Set the order of the effects: each listed effect once.
    fn order(&mut self, _keys: &[u32]) -> Result<()> {
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
fn invalid(detail: &'static str) -> Problem {
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
    fn command(&mut self, command: u32, payload: &[u8]) -> Result<Vec<u8>> {
        let reply = self
            .client
            .exchange(self.nonce, command, payload, COMMAND_TIMEOUT)
            .map_err(|_| invalid("The control mailbox of the camera is not usable."))?
            .ok_or_else(|| {
                Problem::new(503, "runtime_busy", "The camera did not answer. Try again.")
            })?;
        match reply.result {
            0 => Ok(reply.bytes),
            CONTROL_BUSY => Err(Problem::new(
                503,
                "camera_busy",
                "Close the menu on the camera and return to the shooting screen.",
            )),
            CONTROL_CHANGED => Err(Problem::new(
                409,
                "runtime_changed",
                "The camera does not list this Look. Read the Looks again.",
            )),
            CONTROL_UNSUPPORTED => Err(Problem::new(
                503,
                "runtime_protocol_unsupported",
                "This camera has no Film Looks to manage.",
            )),
            _ => Err(invalid("The camera rejected the command.")),
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
    fn looks(&mut self) -> Result<RuntimeLooks> {
        RuntimeLooks::decode(&self.command(CONTROL_LOOKS_READ, &[])?)
    }
    fn turn(&mut self, key: u32, off: bool) -> Result<()> {
        let payload = [key.to_le_bytes(), u32::from(off).to_le_bytes()].concat();
        self.command(CONTROL_LOOK_TURN, &payload).map(drop)
    }
    fn order(&mut self, keys: &[u32]) -> Result<()> {
        let payload: Vec<u8> = keys.iter().flat_map(|key| key.to_le_bytes()).collect();
        self.command(CONTROL_LOOKS_ORDER, &payload).map(drop)
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
    fn looks(&mut self) -> Result<RuntimeLooks> {
        self.session()?.looks()
    }
    fn turn(&mut self, key: u32, off: bool) -> Result<()> {
        self.session()?.turn(key, off)
    }
    fn order(&mut self, keys: &[u32]) -> Result<()> {
        self.session()?.order(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{tests::Memory, Client};

    /// `codes` are the words of the uses, `count` how many uses they hold.
    fn list_of(looks: &[(u32, u32, u32)], selected: u32, codes: &[u32], count: usize) -> Vec<u8> {
        let mut words = vec![looks.len() as u32, selected, 41, count as u32];
        for (key, choice, loaded) in looks {
            words.extend([*key, *choice, *loaded]);
        }
        words.extend(codes);
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }
    fn list(looks: &[(u32, u32, u32)], selected: u32, codes: &[u32]) -> Vec<u8> {
        list_of(looks, selected, codes, codes.len())
    }

    #[test]
    fn uses_name_their_recipe_or_dial_mode() {
        let named =
            7 | 4 << CONTROL_USE_ORIGIN_SHIFT | 2 << CONTROL_USE_NUMBER_SHIFT | CONTROL_USE_NAMED;
        let odd = u32::from(b'O') | u32::from(b'd') << 7 | u32::from(b'd') << 14;
        let dial = 9 | 2 << CONTROL_USE_ORIGIN_SHIFT | CONTROL_USE_DIAL;
        let words = [
            named,
            odd,
            0,
            0,
            0,
            5 | 1 << CONTROL_USE_ORIGIN_SHIFT,
            dial,
            3,
        ];
        let looks = RuntimeLooks::decode(&list_of(&[], 0, &words, 4)).unwrap();
        let origins: Vec<_> = looks
            .uses
            .iter()
            .map(|u| (u.code, u.origin.clone()))
            .collect();
        assert_eq!(
            origins,
            [
                (
                    7,
                    UseOrigin::Recipe {
                        slot: 3,
                        number: 2,
                        name: Some("Odd".into())
                    }
                ),
                (
                    5,
                    UseOrigin::Recipe {
                        slot: 0,
                        number: 0,
                        name: None
                    }
                ),
                (9, UseOrigin::Dial { mode: 2 }),
                (3, UseOrigin::Unknown),
            ]
        );
        // The name words of a named use must follow it.
        assert!(RuntimeLooks::decode(&list_of(&[], 0, &[named, odd], 1)).is_err());
    }

    #[test]
    fn the_effect_list_decodes_and_orders_as_menu_does() {
        let bytes = list(
            &[(10, 0, 1), (11, 2 << 1 | 1, 0), (12, 1 << 1, 1)],
            12,
            &[7, 9],
        );
        let looks = RuntimeLooks::decode(&bytes).unwrap();
        assert_eq!(looks.looks.len(), 3);
        assert_eq!(
            looks.looks[1],
            RuntimeLook {
                key: 11,
                enabled: false,
                position: 2,
                loaded: false
            }
        );
        assert_eq!(looks.selected, Some(12));
        assert_eq!(looks.state_changes, 41);
        assert_eq!(
            looks.uses.iter().map(|u| u.code).collect::<Vec<_>>(),
            [7, 9]
        );
        assert_eq!(looks.menu_order(), [12, 11, 10]);
        assert_eq!(
            RuntimeLooks::decode(&list(&[], 0, &[])).unwrap(),
            RuntimeLooks {
                state_changes: 41,
                ..Default::default()
            }
        );
        for bad in [
            &bytes[..bytes.len() - 4],
            &bytes[..bytes.len() - 1],
            &bytes[..8],
            &list(&[(10, 0, 1), (10, 0, 1)], 0, &[]),
            &list(&[(0, 0, 1)], 0, &[]),
        ] {
            assert!(RuntimeLooks::decode(bad).is_err());
        }
    }

    #[test]
    fn commands_map_the_answers_of_the_camera() {
        let mut memory = Memory::new();
        memory.handler = Box::new(|command, payload| match (command, payload) {
            (CONTROL_LOOKS_READ, []) => (0, list(&[(10, 0, 1)], 0, &[])),
            (CONTROL_LOOK_TURN, [10, 0, 0, 0, 1, 0, 0, 0]) => (0, Vec::new()),
            (CONTROL_LOOK_TURN, [10, 0, 0, 0, 0, 0, 0, 0]) => (CONTROL_BUSY, Vec::new()),
            (CONTROL_LOOK_TURN, _) => (CONTROL_CHANGED, Vec::new()),
            (CONTROL_LOOKS_ORDER, [10, 0, 0, 0]) => (0, Vec::new()),
            (CONTROL_LOOKS_ORDER, _) => (CONTROL_UNSUPPORTED, Vec::new()),
            _ => (CONTROL_INVALID, Vec::new()),
        });
        let mut backend = ControlBackend::new(Client::new(memory).unwrap(), [7; 16]);
        assert_eq!(backend.looks().unwrap().looks[0].key, 10);
        backend.turn(10, true).unwrap();
        assert_eq!(backend.turn(10, false).unwrap_err().code, "camera_busy");
        assert_eq!(backend.turn(11, false).unwrap_err().code, "runtime_changed");
        backend.order(&[10]).unwrap();
        assert_eq!(
            backend.order(&[10, 11]).unwrap_err().code,
            "runtime_protocol_unsupported"
        );
    }

    #[test]
    fn the_key_of_a_look_is_the_state_id_of_its_choice() {
        assert_eq!(
            look_key("portra-400-v1"),
            grkit_core::runtime::state_key_id("CORE.EFFECT.portra-400-v1")
        );
        assert_ne!(look_key("a"), look_key("b"));
    }
}
