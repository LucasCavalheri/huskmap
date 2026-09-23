//! Detail drawer. Slides in from the right; says why a husk can or cannot go.

use freya::animation::*;
use freya::prelude::*;
use huskmap_core::{HuskId, copy};

use crate::components::ui::{agent_mark, caps, display, glyph, hairline, keycap_on, mono};
use crate::icons::{Brand, Glyph};
use crate::theme;
use crate::view_model::{AppState, DrawerView};

#[derive(PartialEq)]
pub struct Drawer {
    pub state: State<AppState>,
    pub view: DrawerView,
    pub brand: Option<Brand>,
}

impl Component for Drawer {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let slide = use_animation_with_dependencies(&self.view.id, |conf, _| {
            conf.on_creation(OnCreation::Run);
            AnimNum::new(1.0, 0.0)
                .time(theme::MOTION_PANEL)
                .function(Function::Cubic)
                .ease(Ease::Out)
        });
        let t = slide.get().value();
        let v = &self.view;
        let tone = theme::tone_color(v.tone);
        let mut state_close = self.state;
        let mut state_mark = self.state;
        let id: HuskId = v.id.clone();

        let mark_mark: SvgViewer = match (self.brand, v.agent) {
            (_, Some(agent)) => agent_mark(Some(agent), theme::BONE, 20.),
            (Some(b), None) => crate::components::ui::brand(b, theme::BONE, 20.),
            (None, None) => glyph(Glyph::for_kind(v.kind), theme::BONE, 20.),
        };

        let mut wards = rect().width(Size::fill()).spacing(10.);
        for (ward, text) in &v.wards {
            let c = match ward {
                huskmap_core::Ward::Orphaned { .. } | huskmap_core::Ward::Warm { .. } => theme::ASH,
                w if w.absolute() => theme::OXBLOOD,
                w if w.blocks() => theme::AMBER,
                _ => theme::COPPER,
            };
            wards = wards.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(10.)
                    .cross_align(Alignment::Center)
                    .child(glyph(crate::icons::Glyph::for_ward(ward), c, 15.))
                    .child(
                        mono(
                            text.clone(),
                            theme::TEXT_MD,
                            theme::mix(c, theme::BONE, 0.35),
                        )
                        .max_lines(2),
                    ),
            );
        }

        let mut facts = rect().width(Size::fill()).spacing(9.);
        for (k, val) in &v.facts {
            facts = facts.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .spacing(12.)
                    .child(rect().width(Size::px(84.)).child(caps(k, theme::DUST)))
                    .child(
                        rect()
                            .content(Content::flex())
                            .width(Size::flex(1.))
                            .child(mono(val.clone(), theme::TEXT_SM, theme::BONE).max_lines(2)),
                    ),
            );
        }

        let verdict = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(10.)
            .cross_align(Alignment::Center)
            .padding((9., 12.))
            .corner_radius(theme::RADIUS)
            .background(theme::mix(theme::CARBON, tone, 0.1))
            .border(
                Border::new()
                    .fill(theme::mix(theme::CARBON, tone, 0.6))
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(7.))
                    .height(Size::px(7.))
                    .corner_radius(4.)
                    .background(tone),
            )
            .child(
                mono(
                    v.verdict.clone(),
                    theme::TEXT_SM,
                    theme::mix(tone, theme::BONE, 0.4),
                )
                .font_weight(FontWeight::MEDIUM),
            );

        // Primary: mark when free, force-mark when only force can lift the guard, else nothing.
        enum Mode {
            Mark,
            Force,
            Locked,
        }
        let mode = if v.can_mark {
            Mode::Mark
        } else if v.can_force {
            Mode::Force
        } else {
            Mode::Locked
        };
        let on = match mode {
            Mode::Mark => v.marked,
            Mode::Force => v.forced,
            Mode::Locked => false,
        };
        let (fill, ink, edge) = match (&mode, on) {
            (Mode::Locked, _) => (theme::CARBON_RAISED, theme::DUST, theme::CARBON_RAISED),
            (Mode::Mark, false) => (theme::COPPER, theme::PITCH, theme::COPPER),
            (Mode::Mark, true) => (theme::CARBON_HOVER, theme::COPPER, theme::COPPER),
            (Mode::Force, false) => (
                theme::CARBON_RAISED,
                theme::mix(theme::OXBLOOD, theme::BONE, 0.3),
                theme::OXBLOOD,
            ),
            (Mode::Force, true) => (theme::OXBLOOD_DEEP, theme::BONE, theme::OXBLOOD),
        };
        let label = match (&mode, on) {
            (Mode::Force, false) => d.force_mark,
            (Mode::Force, true) => d.force_unmark,
            (_, true) => d.unmark,
            _ => d.mark,
        };
        let forcing = matches!(mode, Mode::Force);
        let action = rect()
            .content(Content::flex())
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(10.)
            .padding((13., 16.))
            .corner_radius(theme::RADIUS)
            .background(fill)
            .border(
                Border::new()
                    .fill(edge)
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .on_press(move |_| {
                if forcing {
                    state_mark.write().toggle_force(&id);
                } else {
                    state_mark.write().toggle_mark(&id);
                }
            })
            .child(glyph(
                if forcing { Glyph::Alert } else { Glyph::Mark },
                ink,
                16.,
            ))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(mono(label, theme::TEXT_MD, ink).font_weight(FontWeight::SEMI_BOLD)),
            )
            .child(keycap_on(if forcing { "X" } else { "x" }, ink));

        let tool = |g: Glyph, text: &str| {
            rect()
                .content(Content::flex())
                .horizontal()
                .spacing(8.)
                .cross_align(Alignment::Center)
                .padding((8., 12.))
                .corner_radius(theme::RADIUS)
                .border(
                    Border::new()
                        .fill(theme::HAIRLINE)
                        .width(1.)
                        .alignment(BorderAlignment::Inner),
                )
                .child(glyph(g, theme::ASH, 14.))
                .child(mono(text.to_string(), theme::TEXT_SM, theme::BONE))
        };
        let path_open = v.full_path.clone();
        let path_copy = v.full_path.clone();
        let mut state_open = self.state;
        let mut state_copy = self.state;
        let tools = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(8.)
            .child(tool(Glyph::Folder, d.open_folder).on_press(move |_| {
                if crate::app::open_folder(&path_open).is_err() {
                    state_open.write().status = Some(copy::get().open_failed.into());
                }
            }))
            .child(tool(Glyph::List, d.copy_path).on_press(move |_| {
                let text = path_copy.display().to_string();
                if freya::clipboard::Clipboard::set(text).is_ok() {
                    state_copy.write().status = Some(copy::get().copied.into());
                }
            }));

        let mut body = rect()
            .content(Content::flex())
            .width(Size::fill())
            .padding((24., 26.))
            .spacing(22.)
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .spacing(9.)
                    .child(glyph(Glyph::for_kind(v.kind), theme::COPPER, 14.))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(caps(&v.kind_label, theme::COPPER)),
                    )
                    .child(
                        rect()
                            .content(Content::flex())
                            .padding(4.)
                            .corner_radius(theme::RADIUS)
                            .on_press(move |_| state_close.write().drawer_open = false)
                            .child(glyph(Glyph::Close, theme::ASH, 16.)),
                    ),
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
                            .spacing(12.)
                            .cross_align(Alignment::Center)
                            .child(mark_mark)
                            .child(rect().width(Size::flex(1.)).child(
                                display(v.title.clone(), theme::TITLE_SM, theme::BONE).max_lines(2),
                            )),
                    )
                    .child(mono(v.path.clone(), theme::TEXT_SM, theme::ASH).max_lines(3)),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .cross_align(Alignment::End)
                    .spacing(14.)
                    .child(display(v.size_label.clone(), theme::TITLE_LG, theme::BONE))
                    .child(
                        rect()
                            .content(Content::flex())
                            .padding((0., 0., 10., 0.))
                            .spacing(3.)
                            .child(caps(d.detail_age, theme::DUST))
                            .child(mono(v.age_label.clone(), theme::TEXT_SM, theme::ASH)),
                    ),
            )
            .child(verdict);
        if !v.wards.is_empty() {
            body = body.child(
                rect()
                    .content(Content::flex())
                    .width(Size::fill())
                    .spacing(12.)
                    .child(caps(d.detail_wards, theme::DUST))
                    .child(wards),
            );
        }
        if !v.facts.is_empty() {
            body = body.child(hairline()).child(facts);
        }
        if !v.notes.is_empty() {
            body = body.child(mono(v.notes.join(" · "), theme::TEXT_XS, theme::DUST).max_lines(3));
        }

        rect()
            .content(Content::flex())
            .width(Size::px(theme::DRAWER_WIDTH))
            .height(Size::fill())
            .offset_x(t * theme::DRAWER_WIDTH)
            .opacity(1.0 - t * 0.6)
            .background(theme::CARBON)
            .shadow((-24.0, 0.0, 48.0, 0.0, (0u8, 0u8, 0u8, 180u8)))
            .border(
                Border::new()
                    .fill(theme::HAIRLINE)
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .show_scrollbar(false)
                    .child(body),
            )
            .child({
                let mut foot = rect()
                    .content(Content::flex())
                    .width(Size::fill())
                    .padding((12., 26., 22., 26.))
                    .spacing(10.)
                    .child(tools);
                if forcing {
                    foot = foot.child(mono(d.force_hint, theme::TEXT_XS, theme::DUST).max_lines(2));
                }
                foot.child(action)
            })
    }
}
