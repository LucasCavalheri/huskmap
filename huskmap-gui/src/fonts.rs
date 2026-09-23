//! Embedded faces so the map looks the same on every OS. Both are SIL OFL 1.1 (see assets/fonts).

use crate::theme::{DISPLAY_FACE, MONO_FACE};

/// (family alias, bytes). Several weights share one alias; Skia matches by style.
pub const FONTS: &[(&str, &[u8])] = &[
    (
        DISPLAY_FACE,
        include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf"),
    ),
    (
        DISPLAY_FACE,
        include_bytes!("../assets/fonts/InstrumentSerif-Italic.ttf"),
    ),
    (
        MONO_FACE,
        include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        MONO_FACE,
        include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
    ),
    (
        MONO_FACE,
        include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_are_truetype() {
        assert_eq!(FONTS.len(), 5);
        for (_, bytes) in FONTS {
            assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
        }
    }
}
