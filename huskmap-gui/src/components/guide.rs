//! "How it works": what huskmap is for, how to read the map, and every filter and key.
//! Opens on the first launch and on `?`.

use freya::animation::*;
use freya::prelude::*;
use huskmap_core::copy::{self, GuideItem, GuideMark};

use crate::components::ui::{caps, display, glyph, keycap, keycap_on, mono};
use crate::icons::Glyph;
use crate::theme;
use crate::view_model::{AppState, Tone};

fn tone_of(n: u8) -> Tone {
    match n {
        0 => Tone::Free,
        1 => Tone::Caution,
        2 => Tone::Guarded,
        _ => Tone::Untouchable,
    }
}

fn mark(m: GuideMark, term: &str) -> Element {
    match m {
        GuideMark::Step(n) => rect()
            .width(Size::px(26.))
            .height(Size::px(26.))
            .corner_radius(13.)
            .center()
            .background(theme::mix(theme::carbon(), theme::copper(), 0.25))
            .border(
                Border::new()
                    .fill(theme::copper_deep())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(
                mono(n.to_string(), theme::TEXT_SM, theme::copper()).font_weight(FontWeight::BOLD),
            )
            .into_element(),
        GuideMark::Kind(kind) => {
            glyph(Glyph::for_kind(kind), theme::kind_color(kind), 18.).into_element()
        }
        GuideMark::Tone(n) => rect()
            .width(Size::px(26.))
            .center()
            .child(
                rect()
                    .width(Size::px(12.))
                    .height(Size::px(12.))
                    .corner_radius(6.)
                    .background(theme::tone_color(tone_of(n))),
            )
            .into_element(),
        GuideMark::Key => keycap(term).into_element(),
        GuideMark::Code | GuideMark::Plain => rect()
            .width(Size::px(26.))
            .center()
            .child(
                rect()
                    .width(Size::px(5.))
                    .height(Size::px(5.))
                    .corner_radius(3.)
                    .background(theme::dust()),
            )
            .into_element(),
    }
}

fn item_row(it: &GuideItem) -> Rect {
    let term: Element = match it.mark {
        GuideMark::Key => rect().into_element(),
        GuideMark::Code => rect()
            .padding((3., 8.))
            .corner_radius(theme::RADIUS)
            .background(theme::pitch())
            .border(
                Border::new()
                    .fill(theme::hairline())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .child(mono(it.term, theme::TEXT_SM, theme::amber()))
            .into_element(),
        _ => mono(it.term, theme::TEXT_MD, theme::bone())
            .font_weight(FontWeight::SEMI_BOLD)
            .into_element(),
    };
    let wide_term = matches!(it.mark, GuideMark::Code);
    rect()
        .content(Content::flex())
        .horizontal()
        .width(Size::fill())
        .spacing(14.)
        .cross_align(Alignment::Start)
        .child(
            rect()
                .width(Size::px(if matches!(it.mark, GuideMark::Key) {
                    64.
                } else {
                    30.
                }))
                .cross_align(Alignment::Center)
                .padding((1., 0., 0., 0.))
                .child(mark(it.mark, it.term)),
        )
        .child(
            rect()
                .content(Content::flex())
                .width(Size::flex(1.))
                .spacing(if wide_term { 7. } else { 3. })
                .child(term)
                .child(mono(it.text, theme::TEXT_SM, theme::ash())),
        )
}

#[derive(PartialEq)]
pub struct GuideModal {
    pub state: State<AppState>,
    pub section: usize,
}

impl Component for GuideModal {
    fn render(&self) -> impl IntoElement {
        let d = copy::get();
        let guide = d.guide;
        let enter = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            AnimNum::new(0.0, 1.0)
                .time(theme::MOTION_PANEL)
                .function(Function::Cubic)
                .ease(Ease::Out)
        });
        let t = enter.get().value();
        let at = self.section.min(guide.sections.len().saturating_sub(1));
        let sec = &guide.sections[at];

        let mut nav = rect()
            .content(Content::flex())
            .width(Size::px(236.))
            .height(Size::fill())
            .spacing(2.)
            .padding((26., 14.))
            .background(theme::carbon())
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .spacing(8.)
                    .cross_align(Alignment::Center)
                    .padding((0., 10., 16., 10.))
                    .child(glyph(Glyph::Guide, theme::copper(), 15.))
                    .child(caps(d.guide_open, theme::copper())),
            );
        for (i, s) in guide.sections.iter().enumerate() {
            let mut state = self.state;
            let on = i == at;
            nav = nav.child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .spacing(10.)
                    .cross_align(Alignment::Center)
                    .padding((8., 10.))
                    .corner_radius(theme::RADIUS)
                    .background(if on {
                        Color::from(theme::carbon_hover())
                    } else {
                        Color::TRANSPARENT
                    })
                    .on_press(move |_| state.write().guide_section = i)
                    .child(mono(
                        (i + 1).to_string(),
                        theme::TEXT_XS,
                        if on { theme::copper() } else { theme::dust() },
                    ))
                    .child(
                        mono(
                            s.title,
                            theme::TEXT_SM,
                            if on { theme::bone() } else { theme::ash() },
                        )
                        .max_lines(1),
                    ),
            );
        }

        let mut items = rect()
            .content(Content::flex())
            .width(Size::fill())
            .spacing(16.);
        for it in sec.items {
            items = items.child(item_row(it));
        }
        let last = guide.sections.len() - 1;
        let (mut s_prev, mut s_next, mut s_close) = (self.state, self.state, self.state);
        let ghost = |text: &str| {
            rect()
                .content(Content::flex())
                .padding((9., 14.))
                .corner_radius(theme::RADIUS)
                .border(
                    Border::new()
                        .fill(theme::hairline())
                        .width(1.)
                        .alignment(BorderAlignment::Inner),
                )
                .child(mono(text.to_string(), theme::TEXT_SM, theme::bone()))
        };
        let mut footer = rect()
            .content(Content::flex())
            .horizontal()
            .width(Size::fill())
            .spacing(10.)
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(mono(
                format!("{} / {}", at + 1, last + 1),
                theme::TEXT_XS,
                theme::dust(),
            )));
        if at > 0 {
            footer = footer.child(
                rect()
                    .on_press(move |_| {
                        let mut s = s_prev.write();
                        s.guide_section = s.guide_section.saturating_sub(1);
                    })
                    .child(ghost(&format!("← {}", guide.sections[at - 1].title))),
            );
        }
        if at < last {
            footer = footer.child(
                rect()
                    .on_press(move |_| {
                        let mut s = s_next.write();
                        s.guide_section = (s.guide_section + 1).min(last);
                    })
                    .child(ghost(&format!("{} →", guide.sections[at + 1].title))),
            );
        }
        footer = footer.child(
            rect()
                .content(Content::flex())
                .horizontal()
                .spacing(9.)
                .cross_align(Alignment::Center)
                .padding((9., 16.))
                .corner_radius(theme::RADIUS)
                .background(theme::copper())
                .on_press(move |_| s_close.write().guide_open = false)
                .child(
                    mono(d.guide_close, theme::TEXT_SM, theme::pitch())
                        .font_weight(FontWeight::SEMI_BOLD),
                )
                .child(keycap_on("esc", theme::pitch())),
        );

        let page = rect()
            .content(Content::flex())
            .width(Size::flex(1.))
            .height(Size::fill())
            .padding((30., 34., 22., 34.))
            .spacing(18.)
            .child(caps(guide.title, theme::dust()))
            .child(display(sec.title, theme::TITLE_MD, theme::bone()))
            .child(mono(sec.lead, theme::TEXT_MD, theme::ash()))
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .child(ScrollView::new().expanded().child(items)),
            )
            .child(footer);

        let card = rect()
            .content(Content::flex())
            .horizontal()
            .width(Size::px(980.))
            .height(Size::px(640.))
            .corner_radius(theme::RADIUS + 2.)
            .overflow(Overflow::Clip)
            .background(theme::carbon_raised())
            .border(
                Border::new()
                    .fill(theme::hairline())
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .shadow((0.0, 30.0, 80.0, 0.0, theme::shadow(1.0)))
            .offset_y((1.0 - t) * 18.0)
            .opacity(t)
            .child(nav)
            .child(page);

        rect()
            .content(Content::flex())
            .layer(Layer::OverlayLevel(5))
            .position(Position::new_absolute().top(0.).left(0.))
            .width(Size::percent(100.))
            .height(Size::percent(100.))
            .center()
            .background(theme::scrim(t))
            .child(card)
    }
}
