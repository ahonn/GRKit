//! The effects of the running camera (`runtime/control_looks.h`): their
//! list, and the choices that MENU makes with their checkbox and Order. No
//! command reads the card, loads an effect, or removes one: effect files
//! apply at the next start.
use crate::{
    codec::Codes,
    protocol::Problem,
    runtime_backend::{invalid, RuntimeBackend},
};
use grkit_core::abi::*;

type Result<T> = std::result::Result<T, Problem>;

const CODES: Codes = Codes {
    list: "The effect list",
    changed: "The camera does not list this Look. Read the Looks again.",
    unsupported: "This camera has no Film Looks to manage.",
    invalid: None,
};

/// The effect list of the running camera.
pub fn read(backend: &mut dyn RuntimeBackend) -> Result<RuntimeLooks> {
    RuntimeLooks::decode(&CODES.checked(backend.exchange(CONTROL_LOOKS_READ, &[])?)?)
}

/// Turn an effect on or off, as its checkbox in MENU does.
pub fn turn(backend: &mut dyn RuntimeBackend, key: u32, off: bool) -> Result<()> {
    let payload = [key.to_le_bytes(), u32::from(off).to_le_bytes()].concat();
    CODES
        .checked(backend.exchange(CONTROL_LOOK_TURN, &payload)?)
        .map(drop)
}

/// Set the order of the effects: each listed effect once.
pub fn order(backend: &mut dyn RuntimeBackend, keys: &[u32]) -> Result<()> {
    let payload: Vec<u8> = keys.iter().flat_map(|key| key.to_le_bytes()).collect();
    CODES
        .checked(backend.exchange(CONTROL_LOOKS_ORDER, &payload)?)
        .map(drop)
}

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
            code: word & CONTROL_USE_CODE_MASK,
            origin,
        }
    }
}

/// A recipe name of its name words (`recipes::name_of_words`). None for the
/// automatic name.
pub(crate) fn recipe_name(words: &[u32]) -> Option<String> {
    Some(grkit_core::recipes::name_of_words(words)).filter(|name| !name.is_empty())
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

#[cfg(test)]
impl RuntimeLooks {
    /// The answer of `CONTROL_LOOKS_READ` that decodes to this list.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut words = vec![
            self.looks.len() as u32,
            self.selected.unwrap_or(0),
            self.state_changes,
            self.uses.len() as u32,
        ];
        for look in &self.looks {
            let choice = look.position << 1 | u32::from(!look.enabled);
            words.extend([look.key, choice, u32::from(look.loaded)]);
        }
        for usage in &self.uses {
            match &usage.origin {
                UseOrigin::Unknown => words.push(usage.code),
                UseOrigin::Dial { mode } => {
                    words.push(usage.code | mode << CONTROL_USE_ORIGIN_SHIFT | CONTROL_USE_DIAL)
                }
                UseOrigin::Recipe { slot, number, name } => {
                    let word = usage.code
                        | (slot + 1) << CONTROL_USE_ORIGIN_SHIFT
                        | number << CONTROL_USE_NUMBER_SHIFT;
                    match name {
                        Some(name) => {
                            words.push(word | CONTROL_USE_NAMED);
                            words.extend(grkit_core::recipes::name_words(name));
                        }
                        None => words.push(word),
                    }
                }
            }
        }
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{tests::Memory, Client};
    use crate::runtime_backend::ControlBackend;

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
        let backend = &mut backend;
        assert_eq!(read(backend).unwrap().looks[0].key, 10);
        turn(backend, 10, true).unwrap();
        assert_eq!(turn(backend, 10, false).unwrap_err().code, "camera_busy");
        assert_eq!(
            turn(backend, 11, false).unwrap_err().code,
            "runtime_changed"
        );
        order(backend, &[10]).unwrap();
        assert_eq!(
            order(backend, &[10, 11]).unwrap_err().code,
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
