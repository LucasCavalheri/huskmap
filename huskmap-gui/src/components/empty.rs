//! Illustrated states laid over the idle dial. Dry copy, no "Oops".

use freya::prelude::*;
use huskmap_core::copy;

use crate::components::ui::{caps, display_italic, glyph, keycap_on, mono};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AppState, Intent};

fn scan_button(state: State<AppState>) -> Rect {
    let d = copy::get();
    let mut state = state;
    rect()
        .content(Content::flex())
        .horizontal()
        .spacing(10.)
        .cross_align(Alignment::Center)
        .padding((14., 22.))
        .corner_radius(theme::RADIUS)
        .background(theme::copper())
        .shadow((0.0, 0.0, 40.0, 0.0, theme::rgba(theme::copper(), 90)))
        .on_press(move |_| state.write().request = Some(Intent::Scan))
        .child(glyph(Glyph::Scan, theme::pitch(), 17.))
        .child(
            mono(d.sound_the_grove, theme::TEXT_LG, theme::pitch())
                .font_weight(FontWeight::SEMI_BOLD),
        )
        .child(keycap_on("s", theme::pitch()))
}

fn centered(content: Rect) -> Rect {
    rect()
        .content(Content::flex())
        .layer(Layer::OverlayLevel(1))
        .position(Position::new_absolute().top(0.).left(0.))
        .width(Size::percent(100.))
        .height(Size::percent(100.))
        .center()
        .child(content)
}

fn plate() -> Rect {
    rect()
        .content(Content::flex())
        .width(Size::px(520.))
        .padding((34., 38.))
        .spacing(18.)
        .cross_align(Alignment::Center)
        .corner_radius(theme::RADIUS + 2.)
        .background(theme::rgba(theme::pitch(), 215))
}

pub fn idle(state: State<AppState>) -> Rect {
    let d = copy::get();
    let (first, rest) = d
        .empty_grove
        .split_once(". ")
        .unwrap_or((d.empty_grove, ""));
    centered(
        plate()
            .child(caps(d.map_subtitle, theme::copper()))
            .child(
                display_italic(format!("{first}."), theme::TITLE_MD, theme::bone())
                    .width(Size::fill())
                    .text_align(TextAlign::Center),
            )
            .child(
                mono(rest, theme::TEXT_MD, theme::ash())
                    .width(Size::fill())
                    .text_align(TextAlign::Center)
                    .max_lines(4),
            )
            .child(rect().height(Size::px(6.)))
            .child(scan_button(state)),
    )
}

pub fn failed(state: State<AppState>, message: &str) -> Rect {
    let d = copy::get();
    centered(
        plate()
            .child(glyph(Glyph::Alert, theme::oxblood(), 28.))
            .child(
                display_italic(
                    d.error_grove,
                    theme::TITLE_MD,
                    theme::mix(theme::oxblood(), theme::bone(), 0.3),
                )
                .width(Size::fill())
                .text_align(TextAlign::Center),
            )
            .child(
                mono(message.to_string(), theme::TEXT_SM, theme::ash())
                    .width(Size::fill())
                    .text_align(TextAlign::Center)
                    .max_lines(6),
            )
            .child(scan_button(state)),
    )
}

pub fn clean() -> Rect {
    let d = copy::get();
    centered(
        plate()
            .child(glyph(Glyph::Grove, theme::verdigris(), 28.))
            .child(
                display_italic(d.clean_grove, theme::TITLE_MD, theme::bone())
                    .width(Size::fill())
                    .text_align(TextAlign::Center),
            ),
    )
}
