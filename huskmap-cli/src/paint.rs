//! Afterlife tones for the terminal. Truecolor when stdout is a TTY and `NO_COLOR` is unset.

use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Bone,
    Ash,
    Copper,
    Amber,
    Oxblood,
    Verdigris,
}

impl Tone {
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Bone => (232, 226, 214),
            Self::Ash => (138, 133, 124),
            Self::Copper => (196, 122, 70),
            Self::Amber => (226, 170, 72),
            Self::Oxblood => (190, 58, 62),
            Self::Verdigris => (110, 160, 128),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Paint {
    on: bool,
}

impl Paint {
    pub fn detect() -> Self {
        Self {
            on: std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    pub fn off() -> Self {
        Self { on: false }
    }

    pub fn tone(&self, tone: Tone, text: &str) -> String {
        if !self.on {
            return text.to_string();
        }
        let (r, g, b) = tone.rgb();
        format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_is_plain_on_is_truecolor() {
        assert_eq!(Paint::off().tone(Tone::Amber, "x"), "x");
        let on = Paint { on: true };
        assert_eq!(on.tone(Tone::Bone, "x"), "\x1b[38;2;232;226;214mx\x1b[0m");
        for t in [
            Tone::Ash,
            Tone::Copper,
            Tone::Amber,
            Tone::Oxblood,
            Tone::Verdigris,
        ] {
            assert!(on.tone(t, "y").starts_with("\x1b[38;2;"));
        }
        let _ = Paint::detect();
    }
}
