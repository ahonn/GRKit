//! The shared components, mounted alone on a headless engine.

use std::sync::atomic::{AtomicUsize, Ordering};

use grkit_core::cover::Cover;
use nsh_core::platform::Os;
use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

use super::*;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.environment.os = Os::Web;
    let mut headless = Headless::start(config).unwrap();
    headless.first_render().unwrap();
    headless
}

fn height(headless: &Headless, id: &str) -> f32 {
    let node = headless.screen().id(&format!("id:{id}")).unwrap();
    let layout = headless.screen().layout_nodes();
    layout
        .iter()
        .find(|n| n.id == node)
        .unwrap()
        .bounds
        .size
        .height
        .0
}

/// Whether a view of the test id is laid out; a hairline can be 0 points
/// tall at the headless scale.
fn shows_id(headless: &Headless, id: &str) -> bool {
    headless
        .screen()
        .layout_nodes()
        .iter()
        .any(|n| n.test_id.as_deref() == Some(id) && !n.hidden)
}

static PRESSES: AtomicUsize = AtomicUsize::new(0);

#[component]
fn Rows() -> impl IntoElement {
    view().w(px(390.0)).child(
        list_section([
            list_row("Version").value("1.0.0").test_id("plain"),
            list_row("Start")
                .leading(icon_tile(glyph::POWER, 30.0, TileTone::Camera))
                .page(|| {
                    PRESSES.fetch_add(1, Ordering::SeqCst);
                })
                .test_id("leading"),
            list_row("GR Snap")
                .subtitle("Transfer and remote")
                .busy(true)
                .page(|| {
                    PRESSES.fetch_add(100, Ordering::SeqCst);
                })
                .test_id("two-lines"),
        ])
        .header("About")
        .footer("A note"),
    )
}

#[test]
fn a_section_draws_its_rows_with_separators_between_them() {
    let headless = start(Root::of::<Rows>());
    for copy in ["About", "Version", "1.0.0", "Start", "GR Snap", "A note"] {
        assert!(headless.shows(copy), "{copy}");
    }
    assert_eq!(height(&headless, "plain"), list::ROW_PLAIN);
    assert_eq!(height(&headless, "leading"), list::ROW_LEADING);
    assert_eq!(height(&headless, "two-lines"), list::ROW_TWO_LINES);
    assert!(shows_id(&headless, "plain:separator"));
    assert!(shows_id(&headless, "leading:separator"));
    assert!(!shows_id(&headless, "two-lines:separator"));
}

#[test]
fn a_row_says_its_value_and_a_busy_row_cannot_be_pressed() {
    PRESSES.store(0, Ordering::SeqCst);
    let mut headless = start(Root::of::<Rows>());
    assert!(headless.screen().matches("label:Version, 1.0.0").unwrap());
    headless.press("label:Start").unwrap();
    headless.settle();
    assert_eq!(PRESSES.load(Ordering::SeqCst), 1);
    let busy = headless
        .screen()
        .accessibility_state("label:GR Snap, Transfer and remote")
        .unwrap();
    assert!(busy.busy);
    // A busy row has no press handler: the press finds nothing to run.
    let _ = headless.press("label:GR Snap, Transfer and remote");
    headless.settle();
    assert_eq!(PRESSES.load(Ordering::SeqCst), 1);
}

#[component]
fn Buttons() -> impl IntoElement {
    view()
        .w(px(390.0))
        .child(button("Update", ButtonKind::Primary, ButtonSize::Large, || {}).loading(true))
        .child(button("Add", ButtonKind::Tinted, ButtonSize::Small, || {}).enabled(false))
        .child(
            button("Apply", ButtonKind::Tinted, ButtonSize::Small, || {}).label("Apply Portra 400"),
        )
}

#[test]
fn a_loading_or_disabled_button_cannot_be_pressed() {
    let headless = start(Root::of::<Buttons>());
    let screen = headless.screen();
    let loading = screen.accessibility_state("label:Update").unwrap();
    assert!(loading.busy && loading.disabled);
    assert!(screen.accessibility_state("label:Add").unwrap().disabled);
    assert!(
        !screen
            .accessibility_state("label:Apply Portra 400")
            .unwrap()
            .disabled
    );
}

fn film_cover() -> Cover {
    Cover::for_name("Portra 400")
}

#[component]
fn Covers() -> impl IntoElement {
    view()
        .w(px(390.0))
        .items_start()
        .child(
            view()
                .test_id("film")
                .child(cover_thumb(&film_cover(), 100.0, false)),
        )
}

#[test]
fn a_film_cover_shows_its_box_only() {
    assert!(
        matches!(film_cover(), Cover::Film(_)),
        "the fixture is a film cover"
    );
    assert_eq!(cover_thumb_size(&film_cover(), 100.0), (100.0, 85.0));
    let headless = start(Root::of::<Covers>());
    assert_eq!(height(&headless, "film"), 85.0);
}

/// Each token that the shared components name is in both appearances of
/// the theme.
#[test]
fn the_component_tokens_are_in_both_appearances() {
    let theme: serde_json::Value = serde_json::from_str(include_str!("../../theme.json")).unwrap();
    let mut names = vec![
        "list-background",
        "list-card",
        "list-separator",
        "list-secondary",
        "list-chevron",
        "destructive-text",
        "tint",
        "online",
        "online-dot",
    ];
    for tone in [
        ChipTone::Tint,
        ChipTone::Strong,
        ChipTone::Pending,
        ChipTone::Neutral,
    ] {
        let (fill, ink) = tone.tokens();
        names.push(fill);
        names.extend(ink);
    }
    for appearance in ["light", "dark"] {
        for name in &names {
            assert!(
                theme["cssVars"][appearance][name].is_string(),
                "{name} in {appearance}"
            );
        }
    }
}

fn bounds(headless: &Headless, id: &str) -> nsh_core::geometry::Bounds<nsh_core::Points> {
    headless
        .screen()
        .layout_nodes()
        .into_iter()
        .find(|n| n.test_id.as_deref() == Some(id))
        .unwrap_or_else(|| panic!("{id} is not laid out"))
        .bounds
}

static HERO_OPENS: AtomicUsize = AtomicUsize::new(0);
static CARD_PRESSES: AtomicUsize = AtomicUsize::new(0);

#[component]
fn Hero() -> impl IntoElement {
    view()
        .w(px(358.0))
        .child(
            photo_hero("Film Look · In Use", "Portra 400")
                .cover(film_cover())
                .open(|| {
                    HERO_OPENS.fetch_add(1, Ordering::SeqCst);
                }),
        )
        .child(photo_hero("Image Control", "No recipe or Film Look").empty_mark())
}

#[test]
fn a_hero_says_its_kicker_and_title_and_opens() {
    HERO_OPENS.store(0, Ordering::SeqCst);
    let mut headless = start(Root::of::<Hero>());
    headless
        .press("label:Film Look · In Use, Portra 400")
        .unwrap();
    headless.settle();
    assert_eq!(HERO_OPENS.load(Ordering::SeqCst), 1);
    // A card that opens nothing is no button.
    let screen = headless.screen();
    assert!(
        !screen
            .matches("role:button[Image Control, No recipe or Film Look]")
            .unwrap()
    );
    assert!(screen.shows("No recipe or Film Look"));
}

#[component]
fn Grid() -> impl IntoElement {
    let cell = |name: &str, id: &str| cover_cell(film_cover(), name).test_id(id);
    view().w(px(390.0)).child(cover_grid([
        cell("Portra 400", "c0").caption("In Use", CaptionTone::Current),
        cell("Tri-X 400", "c1"),
        cell("Velvia 100", "c2"),
        cell("Kodachrome 64", "c3").add("Add Kodachrome 64", true, || {}),
    ]))
}

#[test]
fn a_grid_puts_three_equal_cells_in_a_row() {
    let headless = start(Root::of::<Grid>());
    let cells: Vec<_> = ["c0", "c1", "c2", "c3"]
        .map(|id| bounds(&headless, id))
        .into();
    let width = (390.0 - 2.0 * cover_grid::GRID_GAP) / 3.0;
    for cell in &cells[..3] {
        assert!((cell.size.width.0 - width).abs() < 0.5, "{cell:?}");
        assert_eq!(cell.origin.y, cells[0].origin.y);
    }
    assert!(cells[3].origin.y.0 > cells[0].origin.y.0 + cells[0].size.height.0);
    let screen = headless.screen();
    assert!(screen.matches("label:Portra 400, In Use").unwrap());
    let adding = screen
        .accessibility_state("label:Add Kodachrome 64")
        .unwrap();
    assert!(adding.busy && adding.disabled);
}

#[component]
fn FilledCover() -> impl IntoElement {
    view().w(px(120.0)).child(
        view()
            .test_id("fill")
            .child(cover_fill(&film_cover(), false)),
    )
}

#[test]
fn a_filled_cover_keeps_the_box_shape() {
    let headless = start(Root::of::<FilledCover>());
    let fill = bounds(&headless, "fill");
    assert_eq!(fill.size.width.0, 120.0);
    assert!((fill.size.height.0 - 102.0).abs() < 0.5, "{fill:?}");
}

#[component]
fn Cards() -> impl IntoElement {
    view()
        .w(px(358.0))
        .child(
            notice_card("2 changes apply after a restart").action(button(
                "Restart Camera",
                ButtonKind::Primary,
                ButtonSize::Small,
                || {
                    CARD_PRESSES.fetch_add(10, Ordering::SeqCst);
                },
            )),
        )
        .child(
            notice_card("GRKit 1.7.0 waits")
                .subtitle("Confirm on the camera")
                .emphasis()
                .open(|| {
                    CARD_PRESSES.fetch_add(100, Ordering::SeqCst);
                }),
        )
        .child(
            feature_card(view().size(px(10.0)), "Add Your Camera")
                .subtitle("Connect a camera to manage GRKit.")
                .centered()
                .action(button(
                    "Add Camera",
                    ButtonKind::Primary,
                    ButtonSize::Large,
                    || {},
                )),
        )
}

#[test]
fn a_notice_runs_its_button_or_opens_and_a_feature_card_leads_with_its_title() {
    CARD_PRESSES.store(0, Ordering::SeqCst);
    let mut headless = start(Root::of::<Cards>());
    headless.press("label:Restart Camera").unwrap();
    headless.settle();
    headless
        .press("label:GRKit 1.7.0 waits, Confirm on the camera")
        .unwrap();
    headless.settle();
    assert_eq!(CARD_PRESSES.load(Ordering::SeqCst), 110);
    let screen = headless.screen();
    assert!(screen.matches("role:header[Add Your Camera]").unwrap());
    assert!(screen.shows("Connect a camera to manage GRKit."));
    assert!(screen.matches("label:Add Camera").unwrap());
}

#[component]
fn Header() -> impl IntoElement {
    view().w(px(390.0)).child(
        detail_header("Portra 400")
            .subtitle("Built-in · Negative")
            .cover(film_cover())
            .chip(chip("In Use", ChipTone::Tint))
            .photo_button(overlay_button(glyph::PICTURE, "Change Photo", || {})),
    )
}

#[test]
fn a_detail_header_lays_the_cover_across_the_photo_edge() {
    let headless = start(Root::of::<Header>());
    let photo = bounds(&headless, "detail-photo");
    let cover = bounds(&headless, "detail-cover");
    let photo_bottom = photo.origin.y.0 + photo.size.height.0;
    assert!(cover.origin.y.0 < photo_bottom - media::HEADER_SHEET_OVERLAP);
    assert!(cover.origin.y.0 + cover.size.height.0 > photo_bottom);
    let screen = headless.screen();
    assert!(screen.matches("role:header[Portra 400]").unwrap());
    assert!(screen.matches("label:Change Photo").unwrap());
    assert!(screen.shows("In Use"));
}

static CHECKS: AtomicUsize = AtomicUsize::new(0);

#[component]
fn Checks() -> impl IntoElement {
    view()
        .child(check_circle(
            true,
            "Hide Portra 400".into(),
            true,
            false,
            || {
                CHECKS.fetch_add(1, Ordering::SeqCst);
            },
        ))
        .child(check_circle(
            false,
            "Show Delta 100".into(),
            true,
            true,
            || {
                CHECKS.fetch_add(100, Ordering::SeqCst);
            },
        ))
}

#[test]
fn a_check_says_its_state_and_waits_while_it_changes() {
    CHECKS.store(0, Ordering::SeqCst);
    let mut headless = start(Root::of::<Checks>());
    let on = headless
        .screen()
        .accessibility_state("label:Hide Portra 400")
        .unwrap();
    assert_eq!(on.checked, Some(nsh_core::Checked::True));
    headless.press("label:Hide Portra 400").unwrap();
    headless.settle();
    let changing = headless
        .screen()
        .accessibility_state("label:Show Delta 100")
        .unwrap();
    assert!(changing.busy && changing.disabled);
    assert_eq!(CHECKS.load(Ordering::SeqCst), 1);
}
