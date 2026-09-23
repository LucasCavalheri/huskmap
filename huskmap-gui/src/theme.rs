//! Afterlife tokens. Screens must not invent colors, sizes or timings.
//!
//! Plain tuples so the view-model tests never need Skia. Two palettes, dark (Afterlife) and
//! light (Daylight), behind the same token functions.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::view_model::Tone;
use huskmap_core::HuskKind;

pub type Rgb = (u8, u8, u8);
pub type Rgba = (u8, u8, u8, u8);

/// Every color a screen may use. Two palettes share one set of names, so a screen written
/// against `theme::copper()` works in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    // Surfaces, deepest first.
    pub pitch: Rgb,
    pub carbon: Rgb,
    pub carbon_raised: Rgb,
    pub carbon_hover: Rgb,
    pub hairline: Rgb,
    pub hairline_soft: Rgb,
    // Text, strongest first.
    pub bone: Rgb,
    pub ash: Rgb,
    pub dust: Rgb,
    // Accents.
    pub copper: Rgb,
    pub copper_deep: Rgb,
    /// Oxidized copper. Reads as "safe to remove".
    pub verdigris: Rgb,
    pub amber: Rgb,
    pub oxblood: Rgb,
    pub oxblood_deep: Rgb,
    /// Text and icons on a copper button.
    pub on_accent: Rgb,
    /// Text and icons on an oxblood button.
    pub on_danger: Rgb,
}

/// Afterlife: near-black carbon, bone and ash text, oxidized copper. The original.
pub const DARK: Palette = Palette {
    pitch: (7, 7, 6),
    carbon: (13, 13, 11),
    carbon_raised: (20, 19, 17),
    carbon_hover: (29, 27, 23),
    hairline: (44, 40, 34),
    hairline_soft: (30, 28, 24),
    bone: (236, 229, 214),
    ash: (154, 147, 134),
    dust: (96, 91, 82),
    copper: (201, 124, 69),
    copper_deep: (122, 72, 38),
    verdigris: (108, 164, 134),
    amber: (232, 174, 72),
    oxblood: (186, 52, 56),
    oxblood_deep: (74, 20, 23),
    on_accent: (7, 7, 6),
    on_danger: (236, 229, 214),
};

/// Daylight: warm paper and ink, never white. Accents are darker so they hold contrast.
pub const LIGHT: Palette = Palette {
    pitch: (243, 239, 230),
    carbon: (236, 231, 220),
    carbon_raised: (250, 247, 240),
    carbon_hover: (228, 221, 208),
    hairline: (205, 196, 180),
    hairline_soft: (222, 214, 200),
    bone: (30, 27, 22),
    ash: (88, 82, 73),
    dust: (128, 120, 108),
    copper: (168, 90, 38),
    copper_deep: (214, 168, 132),
    verdigris: (46, 116, 86),
    amber: (168, 110, 12),
    oxblood: (168, 36, 42),
    oxblood_deep: (246, 222, 218),
    on_accent: (250, 247, 240),
    on_danger: (250, 247, 240),
};

/// What the person picked. `System` follows the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "light" | "claro" => Self::Light,
            "dark" | "escuro" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    /// Light or dark, given what the desktop prefers.
    pub fn is_light(self, system_light: bool) -> bool {
        match self {
            Self::System => system_light,
            Self::Light => true,
            Self::Dark => false,
        }
    }
}

static LIGHT_ON: AtomicBool = AtomicBool::new(false);

/// Switch the palette every color function reads. The window calls it before each render.
pub fn set_light(on: bool) {
    LIGHT_ON.store(on, Ordering::Relaxed);
}

pub fn is_light() -> bool {
    LIGHT_ON.load(Ordering::Relaxed)
}

pub fn palette() -> &'static Palette {
    if is_light() { &LIGHT } else { &DARK }
}

macro_rules! tokens {
    ($($name:ident),* $(,)?) => {
        $(
            #[inline]
            pub fn $name() -> Rgb {
                palette().$name
            }
        )*
    };
}

tokens!(
    pitch,
    carbon,
    carbon_raised,
    carbon_hover,
    hairline,
    hairline_soft,
    bone,
    ash,
    dust,
    copper,
    copper_deep,
    verdigris,
    amber,
    oxblood,
    oxblood_deep,
    on_accent,
    on_danger,
);

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

/// Shadow color under panels and modals: ink on paper is softer than black on carbon.
pub fn shadow(strength: f32) -> Rgba {
    if is_light() {
        (60, 44, 28, (strength * 90.0) as u8)
    } else {
        (0, 0, 0, (strength * 200.0) as u8)
    }
}

/// The dimmed backdrop behind a modal, `t` = 0..1 as it fades in.
pub fn scrim(t: f32) -> Rgba {
    if is_light() {
        (40, 32, 22, (110.0 * t) as u8)
    } else {
        (0, 0, 0, (170.0 * t) as u8)
    }
}

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
        Tone::Free => verdigris(),
        Tone::Caution => amber(),
        Tone::Guarded => copper(),
        Tone::Untouchable => oxblood(),
    }
}

/// Each kind gets a quiet hue for its sector wash. Risk colors stay reserved for husks.
pub fn kind_color(kind: HuskKind) -> Rgb {
    match kind {
        HuskKind::Worktree => bone(),
        HuskKind::Ballast => copper(),
        HuskKind::Toolchain => mix(copper(), amber(), 0.5),
        HuskKind::Afterimage => ash(),
        HuskKind::Cache => mix(verdigris(), ash(), 0.5),
        HuskKind::Debris => dust(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luma(c: Rgb) -> f32 {
        0.2126 * f32::from(c.0) + 0.7152 * f32::from(c.1) + 0.0722 * f32::from(c.2)
    }

    /// WCAG relative contrast between two colors.
    fn contrast(a: Rgb, b: Rgb) -> f32 {
        let lin = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let l = |c: Rgb| 0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2) + 0.05;
        let (x, y) = (l(a), l(b));
        x.max(y) / x.min(y)
    }

    #[test]
    fn dark_surfaces_are_dark_and_warm() {
        let p = DARK;
        for c in [p.pitch, p.carbon, p.carbon_raised, p.carbon_hover] {
            assert!(luma(c) < 32.0);
            assert!(c.2 <= c.0, "no blue cast");
        }
        assert!(luma(p.pitch) < luma(p.carbon));
        assert!(luma(p.carbon) < luma(p.carbon_raised));
    }

    #[test]
    fn light_surfaces_are_paper_not_white() {
        let p = LIGHT;
        for c in [p.pitch, p.carbon, p.carbon_raised, p.carbon_hover] {
            assert!(luma(c) > 200.0 && luma(c) < 252.0, "{c:?}");
            assert!(c.2 <= c.0, "warm, no blue cast");
        }
    }

    #[test]
    fn text_and_accents_hold_contrast_in_both_palettes() {
        for p in [DARK, LIGHT] {
            assert!(contrast(p.bone, p.pitch) > 12.0);
            assert!(contrast(p.ash, p.pitch) > 4.5, "{:?}", p.ash);
            for accent in [p.copper, p.verdigris, p.amber, p.oxblood] {
                assert!(
                    contrast(accent, p.pitch) > 3.0,
                    "{accent:?} on {:?}",
                    p.pitch
                );
            }
            assert!(contrast(p.on_accent, p.copper) > 3.0);
            assert!(contrast(p.on_danger, p.oxblood) > 4.5);
        }
    }

    #[test]
    fn no_default_blue_or_saas_purple() {
        for p in [DARK, LIGHT] {
            for c in [p.copper, p.verdigris, p.amber, p.oxblood, p.bone, p.ash] {
                assert!(!(c.2 > c.0 && c.2 > c.1), "{c:?} leans blue/purple");
            }
        }
    }

    #[test]
    fn choice_and_switch() {
        assert_eq!(ThemeChoice::parse("Claro"), ThemeChoice::Light);
        assert_eq!(ThemeChoice::parse("dark"), ThemeChoice::Dark);
        assert_eq!(ThemeChoice::parse("whatever"), ThemeChoice::System);
        for c in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
            assert_eq!(ThemeChoice::parse(c.as_str()), c);
        }
        assert_eq!(
            ThemeChoice::System.next().next().next(),
            ThemeChoice::System
        );
        assert!(ThemeChoice::System.is_light(true));
        assert!(!ThemeChoice::System.is_light(false));
        assert!(ThemeChoice::Light.is_light(false));
        assert!(!ThemeChoice::Dark.is_light(true));
        std::thread::spawn(|| {
            set_light(true);
            assert_eq!(pitch(), LIGHT.pitch);
            set_light(false);
            assert_eq!(pitch(), DARK.pitch);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn helpers() {
        assert_eq!(rgba(DARK.amber, 40), (232, 174, 72, 40));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.5), (100, 50, 25));
        assert_eq!(mix(DARK.bone, DARK.pitch, -1.0), DARK.bone);
        assert_eq!(hex((1, 171, 255)), "#01abff");
        assert!(shadow(1.0).3 > 0 && scrim(1.0).3 > scrim(0.5).3);
        assert_ne!(tone_color(Tone::Caution), tone_color(Tone::Guarded));
        let kinds: Vec<Rgb> = HuskKind::all().into_iter().map(kind_color).collect();
        assert_eq!(kinds.len(), 6);
        const { assert!(WINDOW_WIDTH > RAIL_WIDTH + DRAWER_WIDTH + 400.0) };
    }
}
