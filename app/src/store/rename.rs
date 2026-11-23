//! Rename in the platform's own text alert. It closes on Save, so a name the
//! camera refused comes back the next time the alert opens.

use nsh_core::prelude::*;

use super::AppState;
use crate::domain::looks::NAME_MAX;
use crate::platform::{self, NamePrompt};
use crate::strings::tr;

/// What a name was typed for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NameTarget {
    Look(String),
    Recipe(u32),
}

impl AppState {
    pub fn ask_rename_look(self, id: String) {
        let current = self.look(&id).map(|l| l.name).unwrap_or_default();
        self.ask_name(
            NameTarget::Look(id),
            tr("looks.rename_title", &[]),
            tr("looks.name_note", &[]),
            current,
        );
    }

    pub fn ask_rename_recipe(self, id: u32) {
        let current = self.recipe(id).and_then(|r| r.name).unwrap_or_default();
        self.ask_name(
            NameTarget::Recipe(id),
            tr("recipes.rename", &[]),
            tr("recipes.name_note", &[]),
            current,
        );
    }

    fn ask_name(self, target: NameTarget, title: String, message: String, current: String) {
        let value = self
            .name_draft
            .get()
            .filter(|(of, _)| *of == target)
            .map_or(current, |(_, name)| name);
        let prompt = NamePrompt {
            title,
            message,
            value,
            max: NAME_MAX,
            save: tr("looks.save", &[]),
            cancel: tr("looks.cancel", &[]),
        };
        self.root.run(|| {
            spawn(async move {
                let Some(name) = platform::ask_name(prompt).await else {
                    self.name_draft.set(None);
                    return;
                };
                self.name_draft.set(Some((target.clone(), name.clone())));
                match target {
                    NameTarget::Look(id) => self.rename_look(id, name),
                    NameTarget::Recipe(id) => self.rename_recipe(id, name),
                }
            });
        });
    }

    /// The typed name is saved, or its camera is gone.
    pub(super) fn clear_name_draft(self) {
        self.name_draft.set(None);
    }
}
