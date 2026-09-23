//! "A fresher husk is out." Download, verify, install, restart, with the marks kept.

use freya::animation::*;
use freya::prelude::*;
use huskmap_core::copy;
use huskmap_core::settings::Settings;

use crate::components::ui::{caps, display, glyph, keycap, keycap_on, mono};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AppState, Intent, UpdateOffer};

#[derive(PartialEq)]
pub struct UpdateModal {
    pub state: State<AppState>,
    pub offer: UpdateOffer,
    pub updating: bool,
    pub applying: bool,
    pub config_dir: std::path::PathBuf,
}

fn ghost(text: &str) -> Rect {
    rect()
        .content(Content::flex())
        .horizontal()
        .spacing(8.)
        .cross_align(Alignment::Center)
        .padding((10., 14.))
        .corner_radius(theme::RADIUS)
        .border(
            Border::new()
                .fill(theme::HAIRLINE)
                .width(1.)
                .alignment(BorderAlignment::Inner),
        )
        .child(mono(text.to_string(), theme::TEXT_SM, theme::BONE))
}

impl Component for UpdateModal {
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
        let o = &self.offer;
        let (mut later, mut now, mut skip, mut stop) =
            (self.state, self.state, self.state, self.state);
        let (cfg_skip, cfg_stop) = (self.config_dir.clone(), self.config_dir.clone());
        let version_skip = o.version.clone();

        let mut notes = rect()
            .content(Content::flex())
            .width(Size::fill())
            .spacing(6.);
        for line in &o.notes {
            notes = notes.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(9.)
                    .child(mono("·", theme::TEXT_SM, theme::COPPER))
                    .child(mono(line.clone(), theme::TEXT_SM, theme::ASH).max_lines(2)),
            );
        }

        let blocked = self.updating || self.applying;
        let primary_label = if self.updating {
            d.updating.replace("{v}", &o.version)
        } else {
            d.update_now.to_string()
        };
        let primary = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(9.)
            .cross_align(Alignment::Center)
            .padding((11., 18.))
            .corner_radius(theme::RADIUS)
            .background(if blocked {
                theme::CARBON_HOVER
            } else {
                theme::COPPER
            })
            .on_press(move |_| {
                if !blocked {
                    now.write().request = Some(Intent::Update);
                }
            })
            .child(glyph(
                Glyph::Scan,
                if blocked { theme::AMBER } else { theme::PITCH },
                15.,
            ))
            .child(
                mono(
                    primary_label,
                    theme::TEXT_MD,
                    if blocked { theme::AMBER } else { theme::PITCH },
                )
                .font_weight(FontWeight::SEMI_BOLD),
            )
            .child(keycap_on(
                "enter",
                if blocked { theme::AMBER } else { theme::PITCH },
            ));

        let mut card = rect()
            .content(Content::flex())
            .width(Size::px(520.))
            .padding(30.)
            .spacing(18.)
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
            .child(caps(
                d.update_available.replace("{v}", &o.version),
                theme::COPPER,
            ))
            .child(display(d.update_title, theme::TITLE_MD, theme::BONE))
            .child(notes);
        if self.applying {
            card =
                card.child(mono(d.update_during_apply, theme::TEXT_SM, theme::AMBER).max_lines(2));
        }
        card = card
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .spacing(8.)
                    .child(
                        ghost(&d.update_skip.replace("{v}", &o.version)).on_press(move |_| {
                            let v = version_skip.clone();
                            let _ = Settings::update(&cfg_skip, |s| s.skipped_version = Some(v));
                            let mut st = skip.write();
                            st.update = None;
                            st.update_open = false;
                        }),
                    )
                    .child(ghost(d.update_stop).on_press(move |_| {
                        let _ = Settings::update(&cfg_stop, |s| s.check_updates = false);
                        let mut st = stop.write();
                        st.update = None;
                        st.update_open = false;
                    })),
            )
            .child(
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
                            .on_press(move |_| later.write().update_open = false)
                            .child(mono(d.update_later, theme::TEXT_MD, theme::BONE))
                            .child(keycap("esc")),
                    )
                    .child(primary),
            );

        rect()
            .layer(Layer::OverlayLevel(5))
            .position(Position::new_absolute().top(0.).left(0.))
            .width(Size::percent(100.))
            .height(Size::percent(100.))
            .center()
            .background((0u8, 0u8, 0u8, (170.0 * t) as u8))
            .child(card)
    }
}
