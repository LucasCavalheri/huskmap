//! Primitives of the huskmap design system. Screens compose these; they never style text by hand.

use std::cell::RefCell;
use std::collections::HashMap;

use freya::prelude::*;

use crate::icons::{Brand, Glyph, tinted};
use crate::theme::{self, Rgb};

/// Serif display text.
pub fn display(text: impl Into<String>, size: f32, color: Rgb) -> Label {
    label()
        .text(text.into())
        .font_family(theme::DISPLAY_FACE)
        .font_size(size)
        .color(color)
}

/// Italic serif, for the one or two lines per screen that should feel spoken.
pub fn display_italic(text: impl Into<String>, size: f32, color: Rgb) -> Label {
    display(text, size, color).font_slant(FontSlant::Italic)
}

/// Monospace text: paths, bytes, counts, labels.
pub fn mono(text: impl Into<String>, size: f32, color: Rgb) -> Label {
    label()
        .text(text.into())
        .font_family(theme::MONO_FACE)
        .font_size(size)
        .color(color)
}

/// Small uppercase section label.
pub fn caps(text: impl AsRef<str>, color: Rgb) -> Label {
    mono(text.as_ref().to_uppercase(), theme::TEXT_XS, color).font_weight(FontWeight::MEDIUM)
}

pub fn hairline() -> Rect {
    rect()
        .content(Content::flex())
        .width(Size::fill())
        .height(Size::px(1.))
        .background(theme::HAIRLINE_SOFT)
}

thread_local! {
    static TINTED: RefCell<HashMap<(usize, Rgb), &'static [u8]>> = RefCell::new(HashMap::new());
}

/// Tinted SVG bytes, leaked once per (asset, color). The set is small and fixed.
fn tinted_bytes(svg: &'static str, color: Rgb) -> &'static [u8] {
    let key = (svg.as_ptr() as usize, color);
    TINTED.with(|cache| {
        *cache
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| Box::leak(tinted(svg, color).into_bytes().into_boxed_slice()))
    })
}

fn svg_image(svg: &'static str, color: Rgb, size: f32) -> SvgViewer {
    let bytes = tinted_bytes(svg, color);
    SvgViewer::new(((svg.as_ptr() as usize, color, size.to_bits()), bytes))
        .show_loader(false)
        .parallel(false)
        .width(Size::px(size))
        .height(Size::px(size))
}

pub fn glyph(g: Glyph, color: Rgb, size: f32) -> SvgViewer {
    svg_image(g.svg(), color, size)
}

pub fn brand(b: Brand, color: Rgb, size: f32) -> SvgViewer {
    svg_image(b.svg(), color, size)
}

/// Brand mark for an agent, or a neutral glyph for `Unknown`.
pub fn agent_mark(agent: Option<huskmap_core::AgentKind>, color: Rgb, size: f32) -> SvgViewer {
    match agent.and_then(Brand::for_agent) {
        Some(b) => brand(b, color, size),
        None => glyph(Glyph::Occupied, color, size),
    }
}

/// Keycap drawn in `color`, for use on filled buttons.
pub fn keycap_on(key: &str, color: Rgb) -> Rect {
    rect()
        .padding((1., 6.))
        .corner_radius(theme::RADIUS)
        .border(
            Border::new()
                .fill(theme::mix(color, theme::COPPER, 0.35))
                .width(1.)
                .alignment(BorderAlignment::Inner),
        )
        .child(mono(key, theme::TEXT_XS, color))
}

/// Keycap, for hints.
pub fn keycap(key: &str) -> Rect {
    rect()
        .content(Content::flex())
        .padding((1., 6.))
        .corner_radius(theme::RADIUS)
        .border(
            Border::new()
                .fill(theme::HAIRLINE)
                .width(1.)
                .alignment(BorderAlignment::Inner),
        )
        .child(mono(key, theme::TEXT_XS, theme::ASH))
}
