//! Left rail: the weight, the alarms, the legend, the agents, the send button.

use freya::animation::*;
use freya::prelude::*;
use huskmap_core::{HuskId, HuskKind, copy, format_bytes};

use crate::components::ui::{
    agent_mark, caps, display, display_italic, glyph, hairline, mono, rerun_on_change,
};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AlarmRow, AppState, LegendRow};

#[derive(PartialEq)]
pub struct Rail {
    pub state: State<AppState>,
}

/// Big number that counts up whenever a new report lands.
#[derive(PartialEq)]
struct CountUp {
    bytes: u64,
    generation: u64,
}

impl Component for CountUp {
    fn render(&self) -> impl IntoElement {
        let count = use_animation_with_dependencies(&self.generation, |conf, _| {
            rerun_on_change(conf);
            AnimNum::new(0.0, 1.0)
                .time(theme::MOTION_COUNT)
                .function(Function::Expo)
                .ease(Ease::Out)
        });
        let shown = (self.bytes as f64 * f64::from(count.get().value())) as u64;
        let text = format_bytes(shown);
        let (num, unit) = text.split_once(' ').unwrap_or((text.as_str(), ""));
        rect()
            .content(Content::flex())
            .horizontal()
            .cross_align(Alignment::End)
            .spacing(10.)
            .child(display(num.to_string(), 76., theme::bone()))
            .child(
                rect()
                    .content(Content::flex())
                    .padding((0., 0., 16., 0.))
                    .child(display_italic(
                        unit.to_string(),
                        theme::TITLE_MD,
                        theme::copper(),
                    )),
            )
    }
}

#[derive(PartialEq)]
struct AlarmCard {
    row: AlarmRow,
    selected: bool,
    state: State<AppState>,
}

impl Component for AlarmCard {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut state = self.state;
        let id: HuskId = self.row.id.clone();
        let lit = *hovered.read() || self.selected;
        let mut card = rect()
            .content(Content::flex())
            .width(Size::fill())
            .padding((9., 10.))
            .spacing(4.)
            .corner_radius(theme::RADIUS + 3.)
            .background(if lit {
                Color::from(theme::carbon_hover())
            } else {
                Color::TRANSPARENT
            })
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
                    .horizontal()
                    .width(Size::fill())
                    .spacing(9.)
                    .cross_align(Alignment::Center)
                    .child(agent_mark(self.row.agent, theme::bone(), 14.))
                    .child(
                        mono(self.row.title.clone(), theme::TEXT_MD, theme::bone())
                            .font_weight(FontWeight::MEDIUM)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    ),
            );
        for (ward, text) in &self.row.lines {
            let c = match ward {
                huskmap_core::Ward::Occupied { .. } => theme::oxblood(),
                huskmap_core::Ward::Dirty { .. } => theme::amber(),
                _ => theme::copper(),
            };
            card = card.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(8.)
                    .cross_align(Alignment::Center)
                    .child(glyph(Glyph::for_ward(ward), c, 13.))
                    .child(
                        mono(
                            text.clone(),
                            theme::TEXT_SM,
                            theme::mix(c, theme::bone(), 0.35),
                        )
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                    ),
            );
        }
        card
    }

    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.row.id)
    }
}

#[derive(PartialEq)]
struct LegendItem {
    row: LegendRow,
    index: usize,
    state: State<AppState>,
}

impl Component for LegendItem {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut state = self.state;
        let kind: HuskKind = self.row.kind;
        let row = &self.row;
        let text = if row.active {
            theme::bone()
        } else {
            theme::dust()
        };
        rect()
            .content(Content::flex())
            .width(Size::fill())
            .padding((5., 10.))
            .corner_radius(theme::RADIUS)
            .background(if *hovered.read() {
                Color::from(theme::carbon_hover())
            } else {
                Color::TRANSPARENT
            })
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_press(move |_| state.write().set_filter(kind))
            .spacing(6.)
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .spacing(10.)
                    .child(glyph(
                        Glyph::for_kind(kind),
                        if row.active {
                            theme::copper()
                        } else {
                            theme::dust()
                        },
                        15.,
                    ))
                    .child(
                        rect()
                            .content(Content::flex())
                            .width(Size::flex(1.))
                            .child(mono(row.label.clone(), theme::TEXT_MD, text)),
                    )
                    .child(mono(row.count.to_string(), theme::TEXT_XS, theme::dust()))
                    .child(
                        rect()
                            .content(Content::flex())
                            .width(Size::px(76.))
                            .cross_align(Alignment::End)
                            .child(mono(
                                row.bytes_label.clone(),
                                theme::TEXT_MD,
                                if row.active {
                                    theme::ash()
                                } else {
                                    theme::dust()
                                },
                            )),
                    ),
            )
    }

    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.index)
    }
}

impl Component for Rail {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let s = self.state.read();
        let (reclaimable, seen) = s.totals();
        let legend = s.legend();
        let alarms = s.alarms();
        let (marked, marked_bytes) = s.marked_summary();
        let generation = s.generation;
        let selected = s.selected.clone();
        drop(s);

        let mut alarm_list = rect().width(Size::fill()).spacing(8.);
        if alarms.is_empty() {
            alarm_list = alarm_list.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(8.)
                    .cross_align(Alignment::Center)
                    .child(glyph(Glyph::Mark, theme::verdigris(), 13.))
                    .child(mono(d.alarms_none, theme::TEXT_SM, theme::ash()).max_lines(2)),
            );
        } else {
            for row in alarms.iter().take(3) {
                alarm_list = alarm_list.child(AlarmCard {
                    selected: selected.as_ref() == Some(&row.id),
                    row: row.clone(),
                    state: self.state,
                });
            }
            if alarms.len() > 3 {
                alarm_list = alarm_list.child(mono(
                    format!("+{}", alarms.len() - 3),
                    theme::TEXT_SM,
                    theme::dust(),
                ));
            }
        }

        let mut legend_list = rect().width(Size::fill()).spacing(2.);
        for (i, row) in legend.iter().enumerate() {
            legend_list = legend_list.child(LegendItem {
                row: row.clone(),
                index: i,
                state: self.state,
            });
        }

        let mut state = self.state;
        let send_ready = marked > 0 && marked_bytes > 0;
        let send = rect()
            .content(Content::flex())
            .width(Size::fill())
            .padding((14., 16.))
            .corner_radius(theme::RADIUS)
            .background(if send_ready {
                theme::copper()
            } else {
                theme::carbon_raised()
            })
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(10.)
            .on_press(move |_| state.write().open_confirm())
            .child(glyph(
                Glyph::Trash,
                if send_ready {
                    theme::pitch()
                } else {
                    theme::dust()
                },
                16.,
            ))
            .child(
                rect().width(Size::flex(1.)).child(
                    mono(
                        d.apply_open,
                        theme::TEXT_MD,
                        if send_ready {
                            theme::pitch()
                        } else {
                            theme::dust()
                        },
                    )
                    .font_weight(FontWeight::SEMI_BOLD),
                ),
            )
            .child(mono(
                if marked > 0 {
                    format!("{marked} · {}", format_bytes(marked_bytes))
                } else {
                    "0".into()
                },
                theme::TEXT_SM,
                if send_ready {
                    theme::pitch()
                } else {
                    theme::dust()
                },
            ));

        rect()
            .content(Content::flex())
            .width(Size::px(theme::RAIL_WIDTH))
            .height(Size::fill())
            .background(theme::carbon())
            .border(
                Border::new()
                    .fill(theme::hairline_soft())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .show_scrollbar(false)
                    .child(
                        rect()
                            .content(Content::flex())
                            .width(Size::fill())
                            .padding((26., theme::GUTTER, 16., theme::GUTTER))
                            .spacing(22.)
                            .child(
                                rect()
                                    .content(Content::flex())
                                    .spacing(2.)
                                    .child(display_italic(
                                        d.map_title,
                                        theme::TITLE_MD,
                                        theme::bone(),
                                    )),
                            )
                            .child(
                                rect()
                                    .content(Content::flex())
                                    .spacing(4.)
                                    .child(caps(d.reclaimable, theme::dust()))
                                    .child(CountUp {
                                        bytes: reclaimable,
                                        generation,
                                    })
                                    .child(mono(
                                        d.seen_of.replace("{total}", &format_bytes(seen)),
                                        theme::TEXT_SM,
                                        theme::ash(),
                                    )),
                            )
                            .child(
                                rect()
                                    .content(Content::flex())
                                    .width(Size::fill())
                                    .spacing(10.)
                                    .child(
                                        rect()
                                            .content(Content::flex())
                                            .horizontal()
                                            .spacing(8.)
                                            .cross_align(Alignment::Center)
                                            .child(glyph(
                                                Glyph::Alert,
                                                if alarms.is_empty() {
                                                    theme::dust()
                                                } else {
                                                    theme::oxblood()
                                                },
                                                13.,
                                            ))
                                            .child(caps(
                                                d.alarms_title,
                                                if alarms.is_empty() {
                                                    theme::dust()
                                                } else {
                                                    theme::mix(theme::oxblood(), theme::bone(), 0.3)
                                                },
                                            )),
                                    )
                                    .child(alarm_list),
                            )
                            .child(hairline())
                            .child(legend_list),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .padding((12., theme::GUTTER, 18., theme::GUTTER))
                    .child(send),
            )
    }
}
