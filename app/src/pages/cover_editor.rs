//! The cover editor of a Look: the film cover as the site edits it. The page
//! shows the cover as a vector drawing and, beside it, the pixels that the
//! camera shows. A label tile opens as the default film cover of the name.

use grkit_core::cover::{self, Cover, EDITOR_LOOKS, Film, Layout, TEXT_MAX};
use nsh_base::{input, theme};
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use super::look_pages::{TAB_BAR_CLEARANCE, header_button};
use crate::store::{AppState, LookAction};
use crate::strings::tr;
use crate::ui::{action_row_loading, cover_view, footnote, grouped_card, section_title, token};

const PREVIEW: f32 = 150.0;
const LAYOUT_TILE: f32 = 56.0;
const SWATCH_WIDTH: f32 = 40.0;

/// The name under a layout, stronger when it is selected.
fn swatch_label(name: String, selected: bool) -> impl IntoElement {
    text(name)
        .text_size(px(crate::ui::ty::CAPTION))
        .line_height(px(15.0))
        .text_center()
        .font_weight(if selected {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        })
        .text_color(move || {
            if selected {
                theme().foreground
            } else {
                theme().muted_foreground
            }
        })
}

/// What the editor gives a cover: a Look on the camera, or the Look being
/// imported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverTarget {
    Look(String),
    Import,
}

impl CoverTarget {
    /// The film cover the editor opens with: the cover, or the default film
    /// cover of the name.
    fn film(&self, app: AppState) -> Option<Film> {
        let (cover, name) = match self {
            CoverTarget::Look(id) => {
                let look = app.look(id)?;
                (look.shown_cover(), look.name)
            }
            CoverTarget::Import => (app.import_cover()?, app.import_draft()?.name),
        };
        [cover, Cover::for_name(name.trim())]
            .into_iter()
            .find_map(|cover| match cover {
                Cover::Film(film) => Some(film),
                Cover::Label(_) => None,
            })
    }

    fn save(&self, app: AppState, cover: Cover) {
        match self {
            CoverTarget::Look(id) => app.save_look_cover(id.clone(), cover),
            CoverTarget::Import => {
                app.set_import_cover(Some(cover));
                app.back_to_import();
            }
        }
    }

    /// The default cover, when the cover is another one.
    fn default_cover(&self, app: AppState) -> Option<Cover> {
        match self {
            CoverTarget::Look(id) => {
                let look = app.look(id)?;
                look.default_cover
                    .clone()
                    .filter(|cover| *cover != look.shown_cover())
            }
            CoverTarget::Import => {
                app.import_draft()?.cover?;
                app.import_default_cover()
            }
        }
    }

    fn use_default(&self, app: AppState, cover: Cover) {
        match self {
            CoverTarget::Look(id) => app.restore_look_cover(id.clone(), cover),
            CoverTarget::Import => {
                app.set_import_cover(None);
                app.back_to_import();
            }
        }
    }

    /// The cover is being written to the camera.
    fn saving(&self, app: AppState) -> bool {
        matches!(self, CoverTarget::Look(id) if app.look_working(id, LookAction::Cover))
    }

    fn restoring(&self, app: AppState) -> bool {
        matches!(self, CoverTarget::Look(id) if app.look_working(id, LookAction::DefaultCover))
    }

    fn writable(&self, app: AppState) -> bool {
        match self {
            CoverTarget::Look(_) => app.look_busy().is_none() && app.looks_blocked().is_none(),
            CoverTarget::Import => true,
        }
    }
}

/// Each text has 1 to `TEXT_MAX` printable ASCII characters; the number
/// can be left out.
fn valid_texts(film: &Film) -> bool {
    cover::valid_text(&film.brand)
        && cover::valid_text(&film.name)
        && film.number.as_deref().is_none_or(cover::valid_text)
}

fn layout_key(layout: Layout) -> &'static str {
    match layout {
        Layout::Box => "looks.layout.box",
        Layout::Box2 => "looks.layout.box2",
        Layout::Cine => "looks.layout.cine",
        Layout::Stack => "looks.layout.stack",
    }
}

pub fn cover_page(
    app: AppState,
    target: CoverTarget,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    let initial = target.film(app);
    let draft = signal(initial.clone());
    let default_target = target.clone();
    let ready_target = target.clone();
    let save = move || {
        if let Some(film) = draft.get() {
            target.save(app, Cover::Film(film));
        }
    };
    crate::ui::app_page()
        .title(tr("looks.cover", &[]))
        .on_dismiss(on_dismiss)
        .header_right(dynamic(move || {
            let ready = draft
                .get()
                .is_some_and(|film| valid_texts(&film) && Cover::Film(film).fits())
                && ready_target.writable(app);
            let saving = ready_target.saving(app);
            header_button(tr("looks.save", &[]), ready, saving, save.clone())
        }))
        .child(
            view()
                .flex_1()
                .bg(move || token("grouped"))
                .child(initial.map(|_| {
                    crate::ui::page_scroll().flex_1().child(
                        view()
                            .px_6()
                            .pt_4()
                            .pb(px(TAB_BAR_CLEARANCE))
                            .gap_5()
                            .child(preview(draft))
                            .child(layouts(draft))
                            .child(colors(draft))
                            .child(texts(draft))
                            .child(default_cover(app, default_target.clone())),
                    )
                })),
        )
}

/// The action that gives the Look its default cover: for an official Look
/// the cover of the scheme of the card, so that the Look has no cover of its
/// own. Hidden while the Look has that cover.
fn default_cover(app: AppState, target: CoverTarget) -> impl IntoElement {
    dynamic(move || {
        let cover = target.default_cover(app)?;
        let target = target.clone();
        let restoring = target.restoring(app);
        let note = match target {
            CoverTarget::Look(_) => "looks.cover_default_note",
            CoverTarget::Import => "looks.cover_note",
        };
        Some(
            view()
                .gap_2()
                .child(grouped_card(action_row_loading(
                    tr("looks.cover_default", &[]),
                    false,
                    true,
                    restoring,
                    move || target.use_default(app, cover.clone()),
                )))
                .child(footnote(tr(note, &[]))),
        )
    })
}

/// Edit the draft, which exists whenever the editor shows.
fn edit(draft: Signal<Option<Film>>, change: impl FnOnce(&mut Film)) {
    draft.update(|film| {
        if let Some(film) = film {
            change(film);
        }
    });
}

/// The cover large on a gray card, as ADJ shows it.
fn preview(draft: Signal<Option<Film>>) -> impl IntoElement {
    view()
        .items_center()
        .gap(px(10.0))
        .pt(px(24.0))
        .pb(px(16.0))
        .rounded(px(16.0))
        .bg(move || theme().muted)
        .child(dynamic(move || {
            let cover = Cover::Film(draft.get()?);
            Some(
                view()
                    .role(AccessibilityRole::Image)
                    .label(tr("looks.cover_preview", &[]))
                    .child(cover_view(&cover, PREVIEW, false)),
            )
        }))
        .child(
            text(tr("looks.cover_preview", &[]))
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || theme().muted_foreground),
        )
}

/// The brand, film name and number, and what keeps the cover from saving.
fn texts(draft: Signal<Option<Film>>) -> impl IntoElement {
    let field = |title: &'static str,
                 placeholder: Option<&'static str>,
                 capitalize: AutoCapitalize,
                 value: fn(&Film) -> String,
                 set: fn(&mut Film, String),
                 last: bool| {
        let current = move || draft.get().as_ref().map(value).unwrap_or_default();
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .min_h(px(50.0))
            .px_4()
            .style_when(!last, |st| {
                st.border_b_1().border_b_color(move || theme().border)
            })
            .child(
                text(tr(title, &[]))
                    .w(px(96.0))
                    .text_size(px(crate::ui::ty::BODY)),
            )
            .child(
                input()
                    .flex_1()
                    .value(current)
                    .placeholder(placeholder.map(|key| tr(key, &[])).unwrap_or_default())
                    .max_length(TEXT_MAX as u32)
                    .auto_correct(false)
                    .auto_capitalize(capitalize)
                    .label(tr(title, &[]))
                    .text_size(px(crate::ui::ty::BODY))
                    .on_change(move |text: &str| edit(draft, |film| set(film, text.to_owned()))),
            )
            .child(
                text(move || format!("{}/{TEXT_MAX}", current().len()))
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .text_color(move || theme().muted_foreground),
            )
    };
    view()
        .gap_2()
        .child(grouped_card(
            view()
                .child(field(
                    "looks.cover_brand",
                    None,
                    AutoCapitalize::None,
                    |film| film.brand.clone(),
                    |film, text| film.brand = text,
                    false,
                ))
                .child(field(
                    "looks.cover_film",
                    None,
                    AutoCapitalize::Characters,
                    |film| film.name.clone(),
                    |film, text| film.name = text,
                    false,
                ))
                .child(field(
                    "looks.cover_number",
                    Some("looks.cover_optional"),
                    AutoCapitalize::Characters,
                    |film| film.number.clone().unwrap_or_default(),
                    // An empty number shows the name large.
                    |film, text| film.number = (!text.is_empty()).then_some(text),
                    true,
                )),
        ))
        .child(dynamic(move || {
            let film = draft.get()?;
            let (key, problem) = if !valid_texts(&film) {
                ("looks.cover_text_note", true)
            } else if !Cover::Film(film).fits() {
                ("looks.cover_too_wide", true)
            } else {
                ("looks.cover_note", false)
            };
            Some(
                text(tr(key, &[]))
                    .text_size(px(crate::ui::ty::FOOTNOTE))
                    .line_height(px(18.0))
                    .text_color(move || {
                        if problem {
                            theme().destructive
                        } else {
                            theme().muted_foreground
                        }
                    }),
            )
        }))
}

/// The four layouts, each drawn with the draft.
fn layouts(draft: Signal<Option<Film>>) -> impl IntoElement {
    view()
        .gap_2()
        .child(section_title(tr("looks.cover_layout", &[])))
        .child(dynamic(move || {
            let film = draft.get()?;
            let mut row = view().flex_row().gap_2();
            for layout in Layout::ALL {
                let selected = film.layout == layout;
                let tile = Cover::Film(Film {
                    layout,
                    ..film.clone()
                });
                row = row.child(
                    view()
                        .flex_1()
                        .items_center()
                        .gap_2()
                        .child(
                            view()
                                .p(px(5.0))
                                .rounded(px(10.0))
                                .border_2()
                                .border_color(move || {
                                    if selected {
                                        theme().foreground
                                    } else {
                                        token("grouped")
                                    }
                                })
                                .role(AccessibilityRole::Radio)
                                .label(tr(layout_key(layout), &[]))
                                .accessibility_state(move || AccessibilityState {
                                    selected,
                                    ..AccessibilityState::default()
                                })
                                .on_press(move || edit(draft, |film| film.layout = layout))
                                .active(|s| s.opacity(0.72))
                                .child(cover_view(&tile, LAYOUT_TILE, false)),
                        )
                        .child(swatch_label(tr(layout_key(layout), &[]), selected)),
                );
            }
            Some(row)
        }))
}

/// A color set drawn in the blocks of `layout`, as the site draws it
/// (`LayoutSwatch`): the body, the brand band (at the bottom for a stack),
/// bars where the texts go, and the accent strip or stripes. Without text it
/// stays legible at swatch size. The view box is the cover without its
/// transparent rows.
fn swatch_svg(layout: Layout, colors: &cover::Colors) -> String {
    let hex = |[r, g, b]: cover::Rgb| format!("#{r:02x}{g:02x}{b:02x}");
    let stack = layout == Layout::Stack;
    let at = |a: f32, b: f32| if stack { a } else { b };
    let mut rects = vec![
        (0.0, 3.0, 40.0, 34.0, colors.body),
        (0.0, at(28.0, 3.0), 40.0, 9.0, colors.band),
        (11.0, at(31.5, 6.5), 18.0, 2.0, colors.brand),
        (7.0, at(10.0, 16.0), 26.0, 2.5, colors.text),
        (13.0, at(17.0, 22.0), 14.0, 6.0, colors.text),
    ];
    match layout {
        Layout::Box2 => rects.push((0.0, 34.0, 40.0, 3.0, colors.accent)),
        Layout::Cine => {
            rects.extend([32.0, 34.0, 36.0].map(|y| (0.0, y, 40.0, 1.0, colors.accent)))
        }
        _ => {}
    }
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 3 40 34" width="40" height="34">"#,
    );
    for (x, y, width, height, fill) in rects {
        svg.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" fill="{}"/>"#,
            hex(fill)
        ));
    }
    svg.push_str("</svg>");
    svg
}

/// The color sets of the site editor, and the set of the cover when it is
/// another one, each drawn for the layout of the draft, in one row that
/// scrolls sideways. A set shows only its colors; its name is for screen
/// readers.
fn colors(draft: Signal<Option<Film>>) -> impl IntoElement {
    dynamic(move || {
        let film = draft.get()?;
        let current = film.colors();
        let mut looks: Vec<&'static cover::Look> = EDITOR_LOOKS
            .iter()
            .filter_map(|id| cover::find_look(id))
            .collect();
        if let Some(own) = cover::look_of(&current)
            && !looks.iter().any(|look| look.colors() == current)
        {
            looks.insert(0, own);
        }
        let mut row = view().flex_row().gap(px(12.0)).px_6().py(px(6.0));
        for look in looks {
            let colors = look.colors();
            let selected = colors == current;
            let name = tr(&format!("looks.cover_look.{}", look.id), &[]);
            row = row.child(
                view().min_w(px(SWATCH_WIDTH + 16.0)).items_center().child(
                    view()
                        .p(px(3.0))
                        .rounded(px(9.0))
                        .border_2()
                        .border_color(move || {
                            if selected {
                                theme().foreground
                            } else {
                                token("grouped")
                            }
                        })
                        .role(AccessibilityRole::Radio)
                        .label(name)
                        .accessibility_state(move || AccessibilityState {
                            selected,
                            ..AccessibilityState::default()
                        })
                        .on_press(move || {
                            edit(draft, |film| {
                                if film.colors() != colors {
                                    film.colors = Some(colors);
                                }
                            })
                        })
                        .child(
                            view()
                                .rounded(px(4.0))
                                .overflow_hidden()
                                .border_1()
                                .border_color(crate::ui::control_border)
                                .child(
                                    image(ImageSrc::svg_markup(&swatch_svg(film.layout, &colors)))
                                        .w(px(SWATCH_WIDTH))
                                        .h(px(SWATCH_WIDTH * 34.0 / 40.0)),
                                ),
                        ),
                ),
            );
        }
        Some(
            view()
                .gap_2()
                .child(section_title(tr("looks.cover_colors", &[])))
                // The row runs to the edges of the page.
                .child(
                    scroll()
                        .horizontal()
                        .scrollbar(false)
                        .mx(px(-24.0))
                        .child(row),
                ),
        )
    })
}
