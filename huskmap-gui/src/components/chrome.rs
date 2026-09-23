//! Stage top bar and window status bar.

use freya::prelude::*;
use huskmap_core::{Locale, copy};

use crate::components::ui::{glyph, keycap, keycap_on, mono};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AppState, Intent, ViewMode};

fn tab(icon: Glyph, text: &str, active: bool) -> Rect {
    let color = if active { theme::bone() } else { theme::dust() };
    rect()
        .content(Content::flex())
        .horizontal()
        .spacing(8.)
        .cross_align(Alignment::Center)
        .padding((8., 14.))
        .corner_radius(theme::RADIUS)
        .background(if active {
            Color::from(theme::carbon_raised())
        } else {
            Color::TRANSPARENT
        })
        .child(glyph(
            icon,
            if active {
                theme::copper()
            } else {
                theme::dust()
            },
            14.,
        ))
        .child(mono(text.to_string(), theme::TEXT_SM, color).font_weight(FontWeight::MEDIUM))
}

#[derive(PartialEq)]
pub struct TopBar {
    pub state: State<AppState>,
}

impl Component for TopBar {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let mut hover_scan = use_state(|| false);
        let s = self.state.read();
        let mode = s.mode;
        let searching = s.searching;
        let search = s.search.clone();
        let scanning = s.is_scanning();
        let has_report = s.report.is_some();
        let found = s.visible().len();
        drop(s);

        let mut state = self.state;
        let mut state_b = self.state;
        let tabs = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(4.)
            .padding(3.)
            .corner_radius(theme::RADIUS + 1.)
            .border(
                Border::new()
                    .fill(theme::hairline_soft())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .on_press(move |_| state.write().mode = ViewMode::Map)
                    .child(tab(Glyph::Map, d.view_map, mode == ViewMode::Map)),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .on_press(move |_| state_b.write().mode = ViewMode::Ledger)
                    .child(tab(Glyph::List, d.view_ledger, mode == ViewMode::Ledger)),
            );

        let mut state_s = self.state;
        let query = if searching || !search.is_empty() {
            mono(
                if search.is_empty() {
                    "_".to_string()
                } else {
                    format!("{search}_")
                },
                theme::TEXT_MD,
                theme::bone(),
            )
        } else {
            mono(d.search_placeholder, theme::TEXT_MD, theme::dust())
        };
        let search_pill = rect()
            .content(Content::flex())
            .width(Size::px(320.))
            .horizontal()
            .spacing(10.)
            .cross_align(Alignment::Center)
            .padding((9., 14.))
            .corner_radius(theme::RADIUS)
            .background(theme::carbon_raised())
            .border(
                Border::new()
                    .fill(if searching {
                        theme::copper()
                    } else {
                        theme::hairline_soft()
                    })
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .on_press(move |_| {
                let mut s = state_s.write();
                s.searching = true;
            })
            .child(glyph(
                Glyph::Search,
                if searching {
                    theme::copper()
                } else {
                    theme::dust()
                },
                14.,
            ))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(query.max_lines(1).text_overflow(TextOverflow::Ellipsis)),
            )
            .child(if search.is_empty() {
                keycap("/").into_element()
            } else {
                mono(found.to_string(), theme::TEXT_XS, theme::ash()).into_element()
            });

        let mut state_scan = self.state;
        let lit = *hover_scan.read();
        let scan_label = if scanning {
            d.scanning
        } else if has_report {
            d.sound_again
        } else {
            d.sound_the_grove
        };
        let scan = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(9.)
            .cross_align(Alignment::Center)
            .padding((9., 16.))
            .corner_radius(theme::RADIUS)
            .background(if scanning {
                theme::carbon_raised()
            } else if lit {
                theme::mix(theme::copper(), theme::amber(), 0.4)
            } else {
                theme::copper()
            })
            .on_pointer_enter(move |_| hover_scan.set(true))
            .on_pointer_leave(move |_| hover_scan.set(false))
            .on_press(move |_| {
                if !state_scan.peek().is_scanning() {
                    state_scan.write().request = Some(Intent::Scan);
                }
            })
            .child(glyph(
                Glyph::Scan,
                if scanning {
                    theme::amber()
                } else {
                    theme::pitch()
                },
                15.,
            ))
            .child(
                mono(
                    scan_label,
                    theme::TEXT_SM,
                    if scanning {
                        theme::amber()
                    } else {
                        theme::pitch()
                    },
                )
                .font_weight(FontWeight::SEMI_BOLD),
            )
            .child(keycap_on(
                "s",
                if scanning {
                    theme::amber()
                } else {
                    theme::pitch()
                },
            ));

        let mut state_help = self.state;
        let mut hover_help = use_state(|| false);
        let help_lit = *hover_help.read();
        let help = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(8.)
            .cross_align(Alignment::Center)
            .padding((9., 12.))
            .corner_radius(theme::RADIUS)
            .background(if help_lit {
                Color::from(theme::carbon_raised())
            } else {
                Color::TRANSPARENT
            })
            .on_pointer_enter(move |_| hover_help.set(true))
            .on_pointer_leave(move |_| hover_help.set(false))
            .on_press(move |_| state_help.write().open_guide(0))
            .child(glyph(
                Glyph::Help,
                if help_lit {
                    theme::copper()
                } else {
                    theme::ash()
                },
                15.,
            ))
            .child(mono(
                d.guide_open,
                theme::TEXT_SM,
                if help_lit {
                    theme::bone()
                } else {
                    theme::ash()
                },
            ))
            .child(keycap("?"));

        rect()
            .content(Content::flex())
            .width(Size::fill())
            .height(Size::px(theme::TOP_BAR))
            .horizontal()
            .cross_align(Alignment::Center)
            .main_align(Alignment::SpaceBetween)
            .padding((0., theme::GUTTER))
            .child(tabs)
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(12.)
                    .cross_align(Alignment::Center)
                    .child(help)
                    .child(search_pill)
                    .child(scan),
            )
    }
}

/// `"j/k move  x mark"` → `[("j/k", "move"), ("x", "mark")]`.
pub fn keyhints(line: &str) -> Vec<(&str, &str)> {
    line.split("  ")
        .filter_map(|pair| pair.trim().split_once(' '))
        .collect()
}

#[derive(PartialEq)]
pub struct StatusBar {
    pub state: State<AppState>,
    /// Where the language pick is remembered (`$XDG_CONFIG_HOME`).
    pub config_dir: std::path::PathBuf,
}

fn pick_locale(mut state: State<AppState>, config_dir: &std::path::Path, locale: Locale) {
    copy::set_locale(locale);
    if let Err(err) = copy::save_choice(config_dir, locale) {
        tracing::warn!(error = %err, "could not remember the language");
    }
    let mut s = state.write();
    if s.confirm.is_some() {
        // Rebuild the modal's sentences in the new language.
        s.open_confirm();
    }
}

impl Component for StatusBar {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let s = self.state.read();
        let line = s.status_line();
        let scanning = s.is_scanning();
        let offer = s.update.as_ref().map(|u| u.version.clone());
        drop(s);
        let mut state_up = self.state;
        let update_chip = match offer {
            Some(v) => rect()
                .content(Content::flex())
                .horizontal()
                .spacing(7.)
                .cross_align(Alignment::Center)
                .padding((3., 9.))
                .corner_radius(theme::RADIUS)
                .background(theme::mix(theme::carbon(), theme::copper(), 0.18))
                .border(
                    Border::new()
                        .fill(theme::copper_deep())
                        .width(1.)
                        .alignment(BorderAlignment::Inner),
                )
                .on_press(move |_| state_up.write().update_open = true)
                .child(glyph(Glyph::ArrowRight, theme::copper(), 12.))
                .child(mono(
                    d.update_available.replace("{v}", &v),
                    theme::TEXT_XS,
                    theme::copper(),
                ))
                .child(keycap_on("u", theme::copper()))
                .into_element(),
            None => rect().into_element(),
        };
        let locale = copy::current_locale();
        let (state_en, state_pt) = (self.state, self.state);
        let (config_en, config_pt) = (self.config_dir.clone(), self.config_dir.clone());
        let lang = |text: &str, on: bool| {
            rect()
                .content(Content::flex())
                .padding((2., 7.))
                .corner_radius(2.)
                .background(if on {
                    Color::from(theme::carbon_hover())
                } else {
                    Color::TRANSPARENT
                })
                .child(mono(
                    text.to_string(),
                    theme::TEXT_XS,
                    if on { theme::bone() } else { theme::dust() },
                ))
        };
        let current_theme = self.state.read().theme;
        let theme_button = |choice: crate::theme::ThemeChoice, icon: Glyph, label: &str| {
            let mut state = self.state;
            let on = current_theme == choice;
            rect()
                .content(Content::flex())
                .padding((3., 6.))
                .corner_radius(2.)
                .background(if on {
                    Color::from(theme::carbon_hover())
                } else {
                    Color::TRANSPARENT
                })
                .on_press(move |_| state.write().theme = choice)
                .a11y_alt(label.to_string())
                .child(glyph(
                    icon,
                    if on { theme::bone() } else { theme::dust() },
                    12.,
                ))
        };
        let themes = rect()
            .content(Content::flex())
            .horizontal()
            .spacing(2.)
            .cross_align(Alignment::Center)
            .child(theme_button(
                crate::theme::ThemeChoice::System,
                Glyph::System,
                d.theme_system,
            ))
            .child(theme_button(
                crate::theme::ThemeChoice::Light,
                Glyph::Sun,
                d.theme_light,
            ))
            .child(theme_button(
                crate::theme::ThemeChoice::Dark,
                Glyph::Moon,
                d.theme_dark,
            ));
        let mut keys = rect()
            .horizontal()
            .spacing(14.)
            .cross_align(Alignment::Center);
        for (k, label) in keyhints(d.keyhint) {
            keys = keys.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(5.)
                    .cross_align(Alignment::Center)
                    .child(keycap(k))
                    .child(mono(label.to_lowercase(), theme::TEXT_XS, theme::dust())),
            );
        }
        rect()
            .content(Content::flex())
            .width(Size::fill())
            .height(Size::px(theme::STATUS_BAR))
            .horizontal()
            .cross_align(Alignment::Center)
            .main_align(Alignment::SpaceBetween)
            .padding((0., theme::GUTTER))
            .background(theme::pitch())
            .border(
                Border::new()
                    .fill(theme::hairline_soft())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(10.)
                    .cross_align(Alignment::Center)
                    .child(
                        rect()
                            .content(Content::flex())
                            .width(Size::px(6.))
                            .height(Size::px(6.))
                            .corner_radius(3.)
                            .background(if scanning {
                                theme::amber()
                            } else {
                                theme::verdigris()
                            }),
                    )
                    .child(mono(line, theme::TEXT_SM, theme::ash()).max_lines(1)),
            )
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(22.)
                    .cross_align(Alignment::Center)
                    .child(update_chip)
                    .child(keys)
                    .child(themes)
                    .child(
                        rect()
                            .content(Content::flex())
                            .horizontal()
                            .spacing(2.)
                            .cross_align(Alignment::Center)
                            .child(glyph(Glyph::Language, theme::dust(), 13.))
                            .child(rect().width(Size::px(4.)))
                            .child(
                                rect()
                                    .content(Content::flex())
                                    .on_press(move |_| {
                                        pick_locale(state_en, &config_en, Locale::En)
                                    })
                                    .child(lang(d.lang_en, locale == Locale::En)),
                            )
                            .child(
                                rect()
                                    .content(Content::flex())
                                    .on_press(move |_| {
                                        pick_locale(state_pt, &config_pt, Locale::PtBr)
                                    })
                                    .child(lang(d.lang_pt, locale == Locale::PtBr)),
                            ),
                    ),
            )
    }
}
