//! List: the drill-in view. Filter chips on top, sortable columns, virtualized rows in the
//! same order as j/k.

use freya::prelude::*;
use huskmap_core::{HuskId, copy};

use crate::components::ui::{agent_mark, brand, caps, glyph, mono};
use crate::icons::{Brand, Glyph};
use crate::theme;
use crate::view_model::{AppState, Chip, ChipView, FilterBar, LedgerRow, Sort, SortKey, Tone};

const COL_SIZE: f32 = 86.;
const COL_AGE: f32 = 58.;
const COL_WARDS: f32 = 290.;
const GROUP_LABEL: f32 = 116.;

#[derive(PartialEq)]
struct Row {
    row: LedgerRow,
    state: State<AppState>,
}

impl Component for Row {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut state = self.state;
        let mut state_mark = self.state;
        let r = &self.row;
        let id: HuskId = r.id.clone();
        let id_mark = r.id.clone();
        let tone = theme::tone_color(r.tone);
        let bg = if r.selected {
            theme::carbon_hover()
        } else if *hovered.read() {
            theme::carbon_raised()
        } else {
            theme::carbon()
        };
        rect()
            .content(Content::flex())
            .width(Size::fill())
            .height(Size::px(theme::LEDGER_ROW))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(14.)
            .padding((0., 18., 0., 0.))
            .background(bg)
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_press(move |_| {
                let mut s = state.write();
                s.selected = Some(id.clone());
                s.drawer_open = true;
            })
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(3.))
                    .height(Size::fill())
                    .background(if r.selected {
                        Color::from(theme::copper())
                    } else {
                        Color::TRANSPARENT
                    }),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        state_mark.write().toggle_mark(&id_mark);
                    })
                    .width(Size::px(18.))
                    .height(Size::px(18.))
                    .center()
                    .corner_radius(3.)
                    .border(
                        Border::new()
                            .fill(if r.marked {
                                theme::copper()
                            } else {
                                theme::hairline()
                            })
                            .width(1.2)
                            .alignment(BorderAlignment::Inner),
                    )
                    .background(if r.marked {
                        Color::from(theme::copper())
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(if r.marked {
                        glyph(Glyph::Mark, theme::pitch(), 12.).into_element()
                    } else {
                        rect().into_element()
                    }),
            )
            .child(glyph(Glyph::for_kind(r.kind), tone, 16.))
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(COL_SIZE))
                    .cross_align(Alignment::End)
                    .child(
                        mono(r.size_label.clone(), theme::TEXT_MD, theme::bone())
                            .font_weight(FontWeight::MEDIUM),
                    ),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(COL_AGE))
                    .cross_align(Alignment::End)
                    .child(mono(r.age_label.clone(), theme::TEXT_SM, theme::dust())),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::flex(1.))
                    .spacing(2.)
                    .child(
                        mono(r.name.clone(), theme::TEXT_MD, theme::bone())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    )
                    .child(
                        mono(r.path.clone(), theme::TEXT_XS, theme::dust())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    ),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(COL_WARDS))
                    .horizontal()
                    .spacing(8.)
                    .cross_align(Alignment::Center)
                    .child(match (&r.first_ward, r.tone) {
                        (Some(text), _) => mono(
                            if r.ward_count > 1 {
                                format!("{text}  +{}", r.ward_count - 1)
                            } else {
                                text.clone()
                            },
                            theme::TEXT_SM,
                            theme::mix(tone, theme::bone(), 0.2),
                        )
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .into_element(),
                        (None, Tone::Caution) => mono(
                            copy::get().tone_caution,
                            theme::TEXT_SM,
                            theme::mix(theme::amber(), theme::pitch(), 0.2),
                        )
                        .into_element(),
                        (None, _) => rect().into_element(),
                    }),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(18.))
                    .child(match r.brand {
                        Some(b) => brand(b, theme::ash(), 15.).into_element(),
                        None if r.agent.is_some() => {
                            agent_mark(r.agent, theme::ash(), 15.).into_element()
                        }
                        None => rect().into_element(),
                    }),
            )
    }

    /// Siblings need their own keys, or scrolling reuses one row's hooks for another.
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.row.id)
    }
}

#[derive(PartialEq)]
struct ChipButton {
    view: ChipView,
    state: State<AppState>,
}

impl Component for ChipButton {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut state = self.state;
        let v = &self.view;
        let chip = v.chip;
        let lit = *hovered.read();
        let text = if v.active || lit {
            theme::bone()
        } else {
            theme::ash()
        };
        let mut body = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(6.)
            .cross_align(Alignment::Center)
            .padding((4., 10.))
            .corner_radius(theme::RADIUS + 8.)
            .background(if v.active {
                theme::mix(theme::carbon(), theme::copper(), 0.22)
            } else if lit {
                theme::carbon_hover()
            } else {
                theme::carbon_raised()
            })
            .border(
                Border::new()
                    .fill(if v.active {
                        theme::copper_deep()
                    } else {
                        theme::hairline_soft()
                    })
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_press(move |_| state.write().press_chip(chip));
        body = match (chip, v.tone) {
            (_, Some(tone)) => body.child(
                rect()
                    .width(Size::px(7.))
                    .height(Size::px(7.))
                    .corner_radius(4.)
                    .background(theme::tone_color(tone)),
            ),
            (Chip::Kind(kind), _) => body.child(glyph(
                Glyph::for_kind(kind),
                if v.active {
                    theme::copper()
                } else {
                    theme::kind_color(kind)
                },
                12.,
            )),
            (Chip::Agent(agent), _) => body.child(match Brand::for_agent(agent) {
                Some(b) => brand(b, text, 12.).into_element(),
                None => agent_mark(Some(agent), text, 12.).into_element(),
            }),
            _ => body,
        };
        body = body.child(mono(v.label.clone(), theme::TEXT_XS, text));
        if let Some(n) = v.count {
            body = body.child(mono(n.to_string(), theme::TEXT_XS, theme::dust()));
        }
        body
    }

    fn render_key(&self) -> DiffKey {
        DiffKey::from(&format!("{:?}", self.view.chip))
    }
}

fn group(label: &str, chips: &[ChipView], state: State<AppState>) -> Rect {
    let mut wrap = rect()
        .content(Content::Wrap {
            wrap_spacing: Some(6.),
        })
        .horizontal()
        .width(Size::flex(1.))
        .spacing(6.);
    for c in chips {
        wrap = wrap.child(ChipButton {
            view: c.clone(),
            state,
        });
    }
    rect()
        .content(Content::flex())
        .horizontal()
        .width(Size::fill())
        .cross_align(Alignment::Start)
        .child(
            rect()
                .width(Size::px(GROUP_LABEL))
                .padding((6., 0., 0., 0.))
                .child(caps(label, theme::dust())),
        )
        .child(wrap)
}

fn button(text: String, accent: bool) -> Rect {
    rect()
        .content(Content::flex())
        .horizontal()
        .spacing(7.)
        .cross_align(Alignment::Center)
        .padding((6., 12.))
        .corner_radius(theme::RADIUS)
        .background(if accent {
            Color::from(theme::mix(theme::carbon(), theme::copper(), 0.18))
        } else {
            Color::TRANSPARENT
        })
        .border(
            Border::new()
                .fill(if accent {
                    theme::copper_deep()
                } else {
                    theme::hairline()
                })
                .width(1.)
                .alignment(BorderAlignment::Inner),
        )
        .child(mono(
            text,
            theme::TEXT_SM,
            if accent {
                theme::copper()
            } else {
                theme::bone()
            },
        ))
}

fn filter_bar(bar: &FilterBar, state: State<AppState>) -> Rect {
    let d = copy::get();
    let (mut s_mark, mut s_clear, mut s_more) = (state, state, state);
    let more_label = if bar.open {
        d.filters_less.to_string()
    } else if bar.hidden_active > 0 {
        format!("{} · {}", d.filters_more, bar.hidden_active)
    } else {
        d.filters_more.to_string()
    };
    let mut kinds = rect()
        .content(Content::Wrap {
            wrap_spacing: Some(6.),
        })
        .horizontal()
        .width(Size::flex(1.))
        .spacing(6.);
    for c in &bar.kinds {
        kinds = kinds.child(ChipButton {
            view: c.clone(),
            state,
        });
    }
    let mut panel = rect()
        .content(Content::flex())
        .width(Size::fill())
        .spacing(10.)
        .padding((2., 0., 16., 0.))
        .child(
            rect()
                .content(Content::flex())
                .horizontal()
                .width(Size::fill())
                .spacing(12.)
                .cross_align(Alignment::Center)
                .child(kinds)
                .child(
                    rect()
                        .on_press(move |_| {
                            let mut s = s_more.write();
                            s.filters_open = !s.filters_open;
                        })
                        .child(button(more_label, bar.hidden_active > 0 && !bar.open)),
                ),
        );
    if bar.open {
        panel = panel.child(group(d.filter_status, &bar.status, state));
        if !bar.agents.is_empty() {
            panel = panel.child(group(d.filter_agent, &bar.agents, state));
        }
        panel = panel
            .child(group(d.filter_size, &bar.sizes, state))
            .child(group(d.filter_age, &bar.ages, state));
    }
    let mut actions = rect()
        .content(Content::flex())
        .horizontal()
        .spacing(10.)
        .cross_align(Alignment::Center);
    if bar.filtered {
        actions = actions.child(
            rect()
                .on_press(move |_| s_clear.write().clear_filters())
                .child(button(d.filter_clear.into(), false)),
        );
    }
    if let Some((label, _)) = &bar.mark_all {
        actions = actions.child(
            rect()
                .on_press(move |_| s_mark.write().mark_visible())
                .child(button(label.clone(), true)),
        );
    }
    panel.child(
        rect()
            .content(Content::flex())
            .horizontal()
            .width(Size::fill())
            .spacing(14.)
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(mono(
                bar.summary.clone(),
                theme::TEXT_SM,
                theme::ash(),
            )))
            .child(actions),
    )
}

/// A column title that sorts the list. The arrow shows the current order.
fn column(
    label: &str,
    key: SortKey,
    sort: Sort,
    width: Size,
    end: bool,
    state: State<AppState>,
) -> Rect {
    let mut state = state;
    let on = sort.key == key;
    let mut title = rect()
        .content(Content::flex())
        .horizontal()
        .spacing(4.)
        .cross_align(Alignment::Center)
        .on_press(move |_| state.write().toggle_sort(key))
        .child(caps(label, if on { theme::bone() } else { theme::dust() }));
    if on {
        title = title.child(glyph(
            if sort.desc {
                Glyph::SortDown
            } else {
                Glyph::SortUp
            },
            theme::copper(),
            11.,
        ));
    }
    rect()
        .width(width)
        .main_align(if end {
            Alignment::End
        } else {
            Alignment::Start
        })
        .horizontal()
        .child(title)
}

#[derive(PartialEq)]
pub struct Ledger {
    pub state: State<AppState>,
    pub rows: Vec<LedgerRow>,
    pub bar: FilterBar,
    pub sort: Sort,
}

impl Component for Ledger {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let rows = self.rows.clone();
        let state = self.state;
        let sort = self.sort;
        let header = rect()
            .content(Content::flex())
            .width(Size::fill())
            .height(Size::px(34.))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(14.)
            .padding((0., 18., 0., 35.))
            .border(
                Border::new()
                    .fill(theme::hairline_soft())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(rect().width(Size::px(16.)))
            .child(column(
                d.detail_size,
                SortKey::Size,
                sort,
                Size::px(COL_SIZE),
                true,
                state,
            ))
            .child(column(
                d.col_age,
                SortKey::Age,
                sort,
                Size::px(COL_AGE),
                true,
                state,
            ))
            .child(column(
                d.sort_name,
                SortKey::Name,
                sort,
                Size::flex(1.),
                false,
                state,
            ))
            .child(
                rect()
                    .width(Size::px(COL_WARDS))
                    .child(caps(d.detail_wards, theme::dust())),
            )
            .child(rect().width(Size::px(18.)));
        let body = if rows.is_empty() {
            rect()
                .content(Content::flex())
                .width(Size::fill())
                .height(Size::flex(1.))
                .center()
                .spacing(12.)
                .child(glyph(Glyph::Search, theme::dust(), 28.))
                .child(mono(d.search_none, theme::TEXT_MD, theme::ash()))
                .into_element()
        } else {
            let len = rows.len();
            rect()
                .content(Content::flex())
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(
                    VirtualScrollView::new_with_data(rows, move |i, rows: &Vec<LedgerRow>| {
                        Row {
                            row: rows[i].clone(),
                            state,
                        }
                        .into_element()
                    })
                    .length(len)
                    .item_size(theme::LEDGER_ROW)
                    .expanded(),
                )
                .into_element()
        };
        rect()
            .content(Content::flex())
            .expanded()
            .padding((0., theme::GUTTER, 0., theme::GUTTER))
            .child(filter_bar(&self.bar, state))
            .child(header)
            .child(body)
    }
}
