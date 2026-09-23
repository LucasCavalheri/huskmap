//! Apply confirmation. The only door to mutation in the desktop map.

use freya::animation::*;
use freya::prelude::*;
use huskmap_core::copy;

use crate::components::ui::{caps, display, glyph, keycap, keycap_on, mono};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AppState, ConfirmView, Intent};

#[derive(PartialEq)]
pub struct Confirm {
    pub state: State<AppState>,
    pub view: ConfirmView,
}

impl Component for Confirm {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let enter = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            AnimNum::new(0.0, 1.0)
                .time(theme::MOTION_PANEL)
                .function(Function::Cubic)
                .ease(Ease::Out)
        });
        let t = enter.get().value();
        let v = &self.view;
        let mut keep = self.state;
        let mut send = self.state;

        let mut paths = rect().width(Size::fill()).spacing(7.);
        for p in &v.paths {
            paths = paths.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(9.)
                    .cross_align(Alignment::Center)
                    .child(glyph(Glyph::Trash, theme::DUST, 12.))
                    .child(
                        mono(p.clone(), theme::TEXT_SM, theme::ASH)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    ),
            );
        }
        if v.more > 0 {
            paths = paths.child(mono(format!("+{}", v.more), theme::TEXT_SM, theme::DUST));
        }

        let mut card = rect()
            .content(Content::flex())
            .width(Size::px(540.))
            .padding(30.)
            .spacing(20.)
            .corner_radius(theme::RADIUS + 2.)
            .background(theme::CARBON_RAISED)
            .border(
                Border::new()
                    .fill(theme::HAIRLINE)
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .shadow((0.0, 30.0, 80.0, 0.0, (0u8, 0u8, 0u8, 200u8)))
            .offset_y((1.0 - t) * 18.0)
            .opacity(t)
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(9.)
                    .cross_align(Alignment::Center)
                    .child(glyph(Glyph::Alert, theme::OXBLOOD, 14.))
                    .child(caps(
                        d.apply_open,
                        theme::mix(theme::OXBLOOD, theme::BONE, 0.3),
                    )),
            )
            .child(display(v.title.clone(), theme::TITLE_MD, theme::BONE).max_lines(2))
            .child(display(
                v.bytes_label.clone(),
                theme::TITLE_LG,
                theme::COPPER,
            ))
            .child(mono(v.body.clone(), theme::TEXT_MD, theme::ASH).max_lines(4))
            .child(paths);
        if let Some(note) = &v.forced_note {
            card = card.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .spacing(9.)
                    .cross_align(Alignment::Center)
                    .padding((9., 12.))
                    .corner_radius(theme::RADIUS)
                    .background(theme::OXBLOOD_DEEP)
                    .child(glyph(Glyph::Alert, theme::OXBLOOD, 14.))
                    .child(
                        rect().width(Size::flex(1.)).child(
                            mono(
                                note.clone(),
                                theme::TEXT_SM,
                                theme::mix(theme::OXBLOOD, theme::BONE, 0.5),
                            )
                            .max_lines(3),
                        ),
                    ),
            );
        }
        if let Some(note) = &v.guarded_note {
            card = card.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(9.)
                    .cross_align(Alignment::Center)
                    .child(glyph(Glyph::Locked, theme::AMBER, 13.))
                    .child(mono(note.clone(), theme::TEXT_SM, theme::AMBER)),
            );
        }
        card = card.child(
            rect()
                .content(Content::flex())
                .horizontal()
                .width(Size::fill())
                .main_align(Alignment::End)
                .spacing(12.)
                .child(
                    rect()
                        .content(Content::flex())
                        .horizontal()
                        .spacing(9.)
                        .cross_align(Alignment::Center)
                        .padding((11., 16.))
                        .corner_radius(theme::RADIUS)
                        .border(
                            Border::new()
                                .fill(theme::HAIRLINE)
                                .width(1.)
                                .alignment(BorderAlignment::Inner),
                        )
                        .on_press(move |_| keep.write().confirm = None)
                        .child(mono(d.apply_withdraw, theme::TEXT_MD, theme::BONE))
                        .child(keycap("esc")),
                )
                .child(
                    rect()
                        .content(Content::flex())
                        .horizontal()
                        .spacing(9.)
                        .cross_align(Alignment::Center)
                        .padding((11., 18.))
                        .corner_radius(theme::RADIUS)
                        .background(theme::OXBLOOD)
                        .on_press(move |_| send.write().request = Some(Intent::Apply))
                        .child(glyph(Glyph::Trash, theme::BONE, 15.))
                        .child(
                            mono(d.apply_commit, theme::TEXT_MD, theme::BONE)
                                .font_weight(FontWeight::SEMI_BOLD),
                        )
                        .child(keycap_on("enter", theme::BONE)),
                ),
        );

        rect()
            .content(Content::flex())
            .layer(Layer::OverlayLevel(4))
            .position(Position::new_absolute().top(0.).left(0.))
            .width(Size::percent(100.))
            .height(Size::percent(100.))
            .center()
            .background((0u8, 0u8, 0u8, (170.0 * t) as u8))
            .child(card)
    }
}
