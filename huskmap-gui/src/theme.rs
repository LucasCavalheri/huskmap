//! Afterlife tokens. Screens must not invent colors, sizes or timings.
//!
//! Plain tuples so the view-model tests never need Skia. A light theme would be a second
//! `Palette`; v1 ships only this one.

use crate::view_model::Tone;
use huskmap_core::HuskKind;

pub type Rgb = (u8, u8, u8);
pub type Rgba = (u8, u8, u8, u8);

// Surfaces: near-black carbon, warm, never blue.
pub const PITCH: Rgb = (7, 7, 6);
pub const CARBON: Rgb = (13, 13, 11);
pub const CARBON_RAISED: Rgb = (20, 19, 17);
pub const CARBON_HOVER: Rgb = (29, 27, 23);
pub const HAIRLINE: Rgb = (44, 40, 34);
pub const HAIRLINE_SOFT: Rgb = (30, 28, 24);

// Text: bone and ash.
pub const BONE: Rgb = (236, 229, 214);
pub const ASH: Rgb = (154, 147, 134);
pub const DUST: Rgb = (96, 91, 82);

// Accents.
pub const COPPER: Rgb = (201, 124, 69);
pub const COPPER_DEEP: Rgb = (122, 72, 38);
/// Oxidized copper. Reads as "free to go".
pub const VERDIGRIS: Rgb = (108, 164, 134);
pub const AMBER: Rgb = (232, 174, 72);
pub const OXBLOOD: Rgb = (186, 52, 56);
pub const OXBLOOD_DEEP: Rgb = (74, 20, 23);

// Type.
pub const DISPLAY_FACE: &str = "Instrument Serif";
pub const MONO_FACE: &str = "IBM Plex Mono";

pub const TEXT_XS: f32 = 10.5;
pub const TEXT_SM: f32 = 12.0;
pub const TEXT_MD: f32 = 13.5;
pub const TEXT_LG: f32 = 16.0;
pub const TITLE_SM: f32 = 26.0;
pub const TITLE_MD: f32 = 36.0;
pub const TITLE_LG: f32 = 56.0;
pub const TITLE_XL: f32 = 84.0;

// Layout.
pub const WINDOW_WIDTH: f32 = 1480.0;
pub const WINDOW_HEIGHT: f32 = 920.0;
pub const WINDOW_MIN_WIDTH: f32 = 1120.0;
pub const WINDOW_MIN_HEIGHT: f32 = 720.0;
pub const RAIL_WIDTH: f32 = 348.0;
pub const DRAWER_WIDTH: f32 = 420.0;
pub const STATUS_BAR: f32 = 34.0;
pub const TOP_BAR: f32 = 64.0;
pub const LEDGER_ROW: f32 = 44.0;
pub const GUTTER: f32 = 28.0;
pub const RADIUS: f32 = 3.0;

// Motion, in milliseconds. Short and physical.
pub const MOTION_FAST: u64 = 140;
pub const MOTION_PANEL: u64 = 320;
pub const MOTION_REVEAL: u64 = 1400;
pub const MOTION_COUNT: u64 = 1100;
pub const SWEEP_IDLE: u64 = 14_000;
pub const SWEEP_SCAN: u64 = 2_600;
pub const HEARTBEAT: u64 = 1_300;

pub fn rgba(c: Rgb, a: u8) -> Rgba {
    (c.0, c.1, c.2, a)
}

/// Linear mix, `t` = 0 is `a`.
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

pub fn tone_color(tone: Tone) -> Rgb {
    match tone {
        Tone::Free => VERDIGRIS,
        Tone::Caution => AMBER,
        Tone::Guarded => COPPER,
        Tone::Untouchable => OXBLOOD,
    }
}

/// Each kind gets a quiet hue for its sector wash. Risk colors stay reserved for husks.
pub fn kind_color(kind: HuskKind) -> Rgb {
    match kind {
        HuskKind::Worktree => BONE,
        HuskKind::Ballast => COPPER,
        HuskKind::Toolchain => mix(COPPER, AMBER, 0.5),
        HuskKind::Afterimage => ASH,
        HuskKind::Cache => mix(VERDIGRIS, ASH, 0.5),
        HuskKind::Debris => DUST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luma(c: Rgb) -> f32 {
        0.2126 * f32::from(c.0) + 0.7152 * f32::from(c.1) + 0.0722 * f32::from(c.2)
    }

    #[test]
    fn surfaces_are_dark_and_warm() {
        for c in [PITCH, CARBON, CARBON_RAISED, CARBON_HOVER] {
            assert!(luma(c) < 32.0);
            assert!(c.2 <= c.0, "no blue cast");
        }
        assert!(luma(PITCH) < luma(CARBON));
        assert!(luma(CARBON) < luma(CARBON_RAISED));
    }

    #[test]
    fn text_contrast_ladder() {
        assert!(luma(BONE) > luma(ASH));
        assert!(luma(ASH) > luma(DUST));
        assert!(luma(DUST) > luma(CARBON_RAISED) * 2.5);
    }

    #[test]
    fn no_default_blue_or_saas_purple() {
        for c in [COPPER, VERDIGRIS, AMBER, OXBLOOD, BONE, ASH] {
            assert!(!(c.2 > c.0 && c.2 > c.1), "{c:?} leans blue/purple");
        }
    }

    #[test]
    fn helpers() {
        assert_eq!(rgba(AMBER, 40), (232, 174, 72, 40));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.5), (100, 50, 25));
        assert_eq!(mix(BONE, PITCH, -1.0), BONE);
        assert_eq!(hex((1, 171, 255)), "#01abff");
        assert_eq!(tone_color(Tone::Free), VERDIGRIS);
        assert_eq!(tone_color(Tone::Untouchable), OXBLOOD);
        assert_ne!(tone_color(Tone::Caution), tone_color(Tone::Guarded));
        let kinds: Vec<Rgb> = HuskKind::all().into_iter().map(kind_color).collect();
        assert_eq!(kinds.len(), 6);
        const { assert!(WINDOW_WIDTH > RAIL_WIDTH + DRAWER_WIDTH + 400.0) };
    }
}
