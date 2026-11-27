//! Recipes.

use nsh_core::prelude::*;

use super::DemoScreen;
use crate::store::{Route, Tab};

pub(super) const SCREENS: &[DemoScreen] = &[
    // R1 — Recipes list.
    DemoScreen {
        id: "R1",
        key_text: "In Camera Menu",
        alert: false,
        seed: |seed| seed.recipes(),
    },
    // R2 — Recipe on a Film Look.
    DemoScreen {
        id: "R2",
        key_text: "Change Settings on the Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.push(Route::RecipeDetail(1));
        },
    },
    // R3 — Recipe on Image Control.
    DemoScreen {
        id: "R3",
        key_text: "Image Control",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.push(Route::RecipeDetail(3));
        },
    },
    // R4 — a recipe detail.
    DemoScreen {
        id: "R4",
        key_text: "Odd Film",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.push(Route::RecipeDetail(1));
        },
    },
    // R5 — Applied, with Undo.
    DemoScreen {
        id: "R5",
        key_text: "Applied Street",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.apply_recipe(2);
        },
    },
    // R6 — Preset recipes.
    DemoScreen {
        id: "R6",
        key_text: "Presets for GR III.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.open_recipe_presets();
        },
    },
    // R7 — Preset that adds its Film Look.
    DemoScreen {
        id: "R7",
        key_text: "Add to Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.load_catalog();
            app.open_recipe_presets();
            app.push(Route::RecipePreset("cinestill-800t".into()));
        },
    },
    // R8 — Rename a recipe.
    DemoScreen {
        id: "R8",
        key_text: "Rename",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.push(Route::RecipeDetail(1));
            spawn(async move {
                timer(std::time::Duration::from_millis(600)).await;
                app.ask_rename_recipe(1);
            });
        },
    },
    // R9 — Delete a recipe.
    DemoScreen {
        id: "R9",
        key_text: "Delete Odd Film?",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.recipes();
            app.push(Route::RecipeDetail(1));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                crate::pages::confirm_delete_recipe(app, 1, "Odd Film".into());
            });
        },
    },
    // R10 — Camera menu full.
    DemoScreen {
        id: "R10",
        key_text: "Camera Menu Full",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            let demo = seed.demo;
            // Fill the camera menu to its real limit, so this page and R1
            // give the same limit.
            demo.edit_recipes(|f| {
                let on: Vec<_> = f.recipes.iter().filter(|r| r.enabled).cloned().collect();
                let mut id = f.recipes.iter().map(|r| r.id).max().unwrap_or(0);
                for template in on
                    .iter()
                    .cycle()
                    .take(f.max_on.saturating_sub(f.on_count()))
                {
                    id += 1;
                    let mut item = template.clone();
                    item.id = id;
                    item.name = None;
                    item.current = false;
                    f.recipes.insert(f.on_count(), item);
                }
            });
            seed.recipes();
            app.set_recipe_enabled(6, true);
        },
    },
    // R11 — No recipes.
    DemoScreen {
        id: "R11",
        key_text: "No Recipes Yet",
        alert: false,
        seed: |seed| {
            let demo = seed.demo;
            demo.edit_recipes(|f| f.recipes.clear());
            seed.recipes();
        },
    },
    // R12 — The Recipes tab without a camera: the home page shows.
    DemoScreen {
        id: "R12",
        key_text: "Add Your Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_sheet(None);
            app.set_tab(Tab::Recipes);
        },
    },
];
