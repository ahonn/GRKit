//! The cover editor of a Look: the film cover as the site edits it. The page
//! shows the cover as a vector drawing and, beside it, the pixels that the
//! camera shows. A label tile opens as the default film cover of the name.

use std::time::{SystemTime, UNIX_EPOCH};

use grkit_core::cover::{self, Cover, EDITOR_LOOKS, Film, Layout, TEXT_MAX};
use nsh_base::{input, nav_page, theme};
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::look_pages::{TAB_BAR_CLEARANCE, header_button};
use crate::store::AppState;
use crate::strings::tr;
use crate::ui::{
    action_row, camera_icon, cover_view, footnote, grouped_card, section_title, token,
};

const PREVIEW: f32 = 200.0;
const CAMERA_ICON: f32 = 80.0;
const LAYOUT_TILE: f32 = 56.0;
const SWATCH_WIDTH: f32 = 40.0;
const SWATCH_COLUMNS: usize = 5;

/// The name under a layout or color set, stronger when it is selected.
fn swatch_label(name: String, selected: bool) -> impl IntoElement {
    text(name)
        .text_size(px(12.0))
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
            CoverTarget::Look(id) => app.save_look_cover(id.clone(), cover),
            CoverTarget::Import => {
                app.set_import_cover(None);
                app.back_to_import();
            }
        }
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
    nav_page()
        .title(tr("looks.cover", &[]))
        .on_dismiss(on_dismiss)
        .header_right(dynamic(move || {
            let ready = draft
                .get()
                .is_some_and(|film| valid_texts(&film) && Cover::Film(film).fits())
                && ready_target.writable(app);
            header_button(tr("looks.save", &[]), ready, save.clone())
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
                            .child(previews(draft))
                            .child(texts(draft))
                            .child(layouts(draft))
                            .child(colors(draft))
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
        let note = match target {
            CoverTarget::Look(_) => "looks.cover_default_note",
            CoverTarget::Import => "looks.cover_note",
        };
        Some(
            view()
                .gap_2()
                .child(grouped_card(action_row(
                    tr("looks.cover_default", &[]),
                    false,
                    true,
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

/// The vector cover, the camera pixels, and the random action.
fn previews(draft: Signal<Option<Film>>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_4()
        .child(dynamic(move || {
            let cover = Cover::Film(draft.get()?);
            Some(
                view()
                    .role(AccessibilityRole::Image)
                    .label(tr("looks.cover", &[]))
                    .child(cover_view(&cover, PREVIEW, false)),
            )
        }))
        .child(
            view()
                .items_center()
                .gap_2()
                .child(dynamic(move || {
                    let cover = Cover::Film(draft.get()?);
                    Some(
                        view()
                            .role(AccessibilityRole::Image)
                            .label(tr("looks.cover_on_camera", &[]))
                            .child(camera_icon(&cover, CAMERA_ICON)),
                    )
                }))
                .child(
                    text(tr("looks.cover_on_camera", &[]))
                        .text_size(px(13.0))
                        .text_center()
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    view()
                        .mt_2()
                        .h(px(32.0))
                        .px(px(14.0))
                        .rounded_full()
                        .items_center()
                        .justify_center()
                        .bg(move || theme().muted)
                        .role(AccessibilityRole::Button)
                        .label(tr("looks.cover_random", &[]))
                        .on_press(move || {
                            let seed = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map_or(0, |time| time.as_nanos() as u64);
                            edit(draft, |film| film.shuffle(seed));
                        })
                        .active(|s| s.opacity(0.72))
                        .child(
                            text(tr("looks.cover_random", &[]))
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(move || token("link")),
                        ),
                ),
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
            .child(text(tr(title, &[])).w(px(96.0)).text_size(px(17.0)))
            .child(
                input()
                    .flex_1()
                    .value(current)
                    .placeholder(placeholder.map(|key| tr(key, &[])).unwrap_or_default())
                    .max_length(TEXT_MAX as u32)
                    .auto_correct(false)
                    .auto_capitalize(capitalize)
                    .label(tr(title, &[]))
                    .text_size(px(17.0))
                    .on_change(move |text: &str| edit(draft, |film| set(film, text.to_owned()))),
            )
            .child(
                text(move || format!("{}/{TEXT_MAX}", current().len()))
                    .text_size(px(15.0))
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
                    .text_size(px(13.0))
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
                                        token("tint")
                                    } else {
                                        theme().border
                                    }
                                })
                                .bg(move || theme().card)
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
/// another one, each drawn for the layout of the draft.
fn colors(draft: Signal<Option<Film>>) -> impl IntoElement {
    view()
        .gap_2()
        .child(section_title(tr("looks.cover_colors", &[])))
        .child(dynamic(move || {
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
            let mut cells = Vec::new();
            for look in looks {
                let colors = look.colors();
                let selected = colors == current;
                let name = tr(&format!("looks.cover_look.{}", look.id), &[]);
                cells.push(
                    view()
                        .items_center()
                        .gap(px(5.0))
                        .child(
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
                                .label(name.clone())
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
                                        .border_color(move || theme().border)
                                        .child(
                                            image(ImageSrc::svg_markup(&swatch_svg(
                                                film.layout,
                                                &colors,
                                            )))
                                            .w(px(SWATCH_WIDTH))
                                            .h(px(SWATCH_WIDTH * 34.0 / 40.0)),
                                        ),
                                ),
                        )
                        .child(swatch_label(name, selected))
                        .into_element(),
                );
            }
            let mut grid = view().gap_3();
            let mut cells = cells.into_iter().peekable();
            while cells.peek().is_some() {
                let mut row = view().flex_row().gap(px(6.0));
                for _ in 0..SWATCH_COLUMNS {
                    row = row.child(view().flex_1().child(cells.next()));
                }
                grid = grid.child(row);
            }
            Some(grid)
        }))
}
