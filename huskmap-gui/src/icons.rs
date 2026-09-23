//! One table for every glyph and brand mark. See AGENTS.md, "Icons".
//!
//! UI glyphs are Hugeicons (stroke, 1.5). Agents and toolchains wear their real marks.
//! Assets ship white; [`tinted`] swaps the color in the bytes so any token can paint them.

use huskmap_core::{AgentKind, Ecosystem, Husk, HuskKind, Ward};

use crate::theme::{Rgb, hex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glyph {
    Worktree,
    Ballast,
    Toolchain,
    Afterimage,
    Cache,
    Debris,
    Occupied,
    Dirty,
    Stranded,
    Locked,
    Warm,
    Secrets,
    Orphaned,
    NoGit,
    Scan,
    Search,
    Mark,
    Trash,
    Close,
    ArrowRight,
    Chevron,
    Alert,
    Grove,
    List,
    Map,
    Clock,
    Folder,
    Keyboard,
    Language,
    Disk,
    Help,
    Filter,
    SortDown,
    SortUp,
    Guide,
    Sun,
    Moon,
    System,
}

impl Glyph {
    pub fn svg(self) -> &'static str {
        macro_rules! icon {
            ($name:literal) => {
                include_str!(concat!("../assets/icons/", $name, ".svg"))
            };
        }
        match self {
            Self::Worktree => icon!("worktree"),
            Self::Ballast => icon!("ballast"),
            Self::Toolchain => icon!("toolchain"),
            Self::Afterimage => icon!("afterimage"),
            Self::Cache => icon!("cache"),
            Self::Debris => icon!("debris"),
            Self::Occupied => icon!("occupied"),
            Self::Dirty => icon!("dirty"),
            Self::Stranded => icon!("stranded"),
            Self::Locked => icon!("locked"),
            Self::Warm => icon!("warm"),
            Self::Secrets => icon!("secrets"),
            Self::Orphaned => icon!("orphaned"),
            Self::NoGit => icon!("no-git"),
            Self::Scan => icon!("scan"),
            Self::Search => icon!("search"),
            Self::Mark => icon!("mark"),
            Self::Trash => icon!("trash"),
            Self::Close => icon!("close"),
            Self::ArrowRight => icon!("arrow-right"),
            Self::Chevron => icon!("chevron-right"),
            Self::Alert => icon!("alert"),
            Self::Grove => icon!("grove"),
            Self::List => icon!("list"),
            Self::Map => icon!("map"),
            Self::Clock => icon!("clock"),
            Self::Folder => icon!("folder"),
            Self::Keyboard => icon!("keyboard"),
            Self::Language => icon!("language"),
            Self::Disk => icon!("disk"),
            Self::Help => icon!("help"),
            Self::Filter => icon!("filter"),
            Self::SortDown => icon!("sort-down"),
            Self::SortUp => icon!("sort-up"),
            Self::Guide => icon!("guide"),
            Self::Sun => icon!("sun"),
            Self::Moon => icon!("moon"),
            Self::System => icon!("system"),
        }
    }

    pub fn for_kind(kind: HuskKind) -> Self {
        match kind {
            HuskKind::Worktree => Self::Worktree,
            HuskKind::Ballast => Self::Ballast,
            HuskKind::Toolchain => Self::Toolchain,
            HuskKind::Afterimage => Self::Afterimage,
            HuskKind::Cache => Self::Cache,
            HuskKind::Debris => Self::Debris,
        }
    }

    pub fn for_ward(ward: &Ward) -> Self {
        match ward {
            Ward::Occupied { .. } | Ward::NeighborBusy { .. } => Self::Occupied,
            Ward::Dirty { .. } => Self::Dirty,
            Ward::Stranded { .. } => Self::Stranded,
            Ward::Locked { .. } | Ward::Primary => Self::Locked,
            Ward::Secrets => Self::Secrets,
            Ward::OutsideRoots => Self::Folder,
            Ward::Warm { .. } => Self::Warm,
            Ward::NoGit => Self::NoGit,
            Ward::Orphaned { .. } => Self::Orphaned,
        }
    }
}

/// Real brand marks, never redrawn. See `assets/brands/SOURCES.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Brand {
    Claude,
    OpenAi,
    Cursor,
    Gemini,
    OpenCode,
    Aider,
    Grok,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Python,
    Uv,
    Rust,
    Go,
    Playwright,
    Git,
}

impl Brand {
    pub fn svg(self) -> &'static str {
        macro_rules! mark {
            ($name:literal) => {
                include_str!(concat!("../assets/brands/", $name, ".svg"))
            };
        }
        match self {
            Self::Claude => mark!("claude"),
            Self::OpenAi => mark!("openai"),
            Self::Cursor => mark!("cursor"),
            Self::Gemini => mark!("gemini"),
            Self::OpenCode => mark!("opencode"),
            Self::Aider => mark!("aider"),
            Self::Grok => mark!("grok"),
            Self::Npm => mark!("npm"),
            Self::Pnpm => mark!("pnpm"),
            Self::Yarn => mark!("yarn"),
            Self::Bun => mark!("bun"),
            Self::Python => mark!("python"),
            Self::Uv => mark!("uv"),
            Self::Rust => mark!("rust"),
            Self::Go => mark!("go"),
            Self::Playwright => mark!("playwright"),
            Self::Git => mark!("git"),
        }
    }

    /// `Unknown` has no brand; callers fall back to a neutral glyph.
    pub fn for_agent(agent: AgentKind) -> Option<Self> {
        match agent {
            AgentKind::Claude => Some(Self::Claude),
            AgentKind::Codex => Some(Self::OpenAi),
            AgentKind::Cursor => Some(Self::Cursor),
            AgentKind::Gemini => Some(Self::Gemini),
            AgentKind::OpenCode => Some(Self::OpenCode),
            AgentKind::Aider => Some(Self::Aider),
            AgentKind::Grok => Some(Self::Grok),
            AgentKind::Unknown => None,
        }
    }

    /// Toolchain husks are named after the tool; the path tells which one.
    pub fn for_tool_path(path: &std::path::Path) -> Option<Self> {
        let p = path
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace('\\', "/");
        let table: [(&str, Self); 14] = [
            ("/.npm/", Self::Npm),
            ("npm-cache", Self::Npm),
            ("pnpm", Self::Pnpm),
            ("yarn", Self::Yarn),
            ("/.bun/", Self::Bun),
            ("/pip", Self::Python),
            ("pypoetry", Self::Python),
            ("/uv", Self::Uv),
            ("/.cargo/", Self::Rust),
            ("go-build", Self::Go),
            ("ms-playwright", Self::Playwright),
            ("/.venv", Self::Python),
            ("/node_modules", Self::Npm),
            ("/target", Self::Rust),
        ];
        let p = format!("{p}/");
        table
            .iter()
            .find(|(needle, _)| p.contains(needle))
            .map(|(_, b)| *b)
    }

    pub fn for_ecosystem(eco: Ecosystem) -> Option<Self> {
        match eco {
            Ecosystem::Node => Some(Self::Npm),
            Ecosystem::Python => Some(Self::Python),
            Ecosystem::Rust => Some(Self::Rust),
            Ecosystem::Go => Some(Self::Go),
            Ecosystem::Browsers => Some(Self::Playwright),
        }
    }

    /// The mark that best identifies a husk: its agent, else its tool, else its ecosystem.
    pub fn for_husk(husk: &Husk) -> Option<Self> {
        husk.agent
            .and_then(Self::for_agent)
            .or_else(|| {
                (husk.kind == HuskKind::Toolchain)
                    .then(|| Self::for_tool_path(&husk.path))
                    .flatten()
            })
            .or_else(|| husk.ecosystem.and_then(Self::for_ecosystem))
    }
}

/// The SVG with its white paint replaced by `color`.
pub fn tinted(svg: &str, color: Rgb) -> String {
    let c = hex(color);
    svg.replace("\"white\"", &format!("\"{c}\""))
        .replace("'white'", &format!("'{c}'"))
        .replace("#FFFFFF", &c)
        .replace("#ffffff", &c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const GLYPHS: [Glyph; 38] = [
        Glyph::Worktree,
        Glyph::Ballast,
        Glyph::Toolchain,
        Glyph::Afterimage,
        Glyph::Cache,
        Glyph::Debris,
        Glyph::Occupied,
        Glyph::Dirty,
        Glyph::Stranded,
        Glyph::Locked,
        Glyph::Warm,
        Glyph::Secrets,
        Glyph::Orphaned,
        Glyph::NoGit,
        Glyph::Scan,
        Glyph::Search,
        Glyph::Mark,
        Glyph::Trash,
        Glyph::Close,
        Glyph::ArrowRight,
        Glyph::Chevron,
        Glyph::Alert,
        Glyph::Grove,
        Glyph::List,
        Glyph::Map,
        Glyph::Clock,
        Glyph::Folder,
        Glyph::Keyboard,
        Glyph::Language,
        Glyph::Disk,
        Glyph::Help,
        Glyph::Filter,
        Glyph::SortDown,
        Glyph::SortUp,
        Glyph::Guide,
        Glyph::Sun,
        Glyph::Moon,
        Glyph::System,
    ];

    const BRANDS: [Brand; 17] = [
        Brand::Claude,
        Brand::OpenAi,
        Brand::Cursor,
        Brand::Gemini,
        Brand::OpenCode,
        Brand::Aider,
        Brand::Grok,
        Brand::Npm,
        Brand::Pnpm,
        Brand::Yarn,
        Brand::Bun,
        Brand::Python,
        Brand::Uv,
        Brand::Rust,
        Brand::Go,
        Brand::Playwright,
        Brand::Git,
    ];

    #[test]
    fn every_asset_is_svg_and_tintable() {
        for svg in GLYPHS
            .iter()
            .map(|g| g.svg())
            .chain(BRANDS.iter().map(|b| b.svg()))
        {
            assert!(svg.contains("<svg") && svg.contains("viewBox"));
            let t = tinted(svg, (1, 2, 3));
            assert!(t.contains("#010203"), "{}", &svg[..80.min(svg.len())]);
            assert!(!t.contains("\"white\""));
        }
    }

    #[test]
    fn stroke_weight_is_one_family() {
        for g in GLYPHS {
            let svg = g.svg();
            assert!(!svg.contains("stroke-width=\"2\""), "{g:?}");
        }
    }

    #[test]
    fn agents_wear_their_own_marks() {
        assert_eq!(Brand::for_agent(AgentKind::Claude), Some(Brand::Claude));
        assert_eq!(Brand::for_agent(AgentKind::Codex), Some(Brand::OpenAi));
        assert_eq!(Brand::for_agent(AgentKind::Unknown), None);
        for a in AgentKind::KNOWN {
            assert!(Brand::for_agent(a).is_some(), "{a:?}");
        }
        assert!(Brand::Claude.svg().contains("Claude"));
    }

    #[test]
    fn tools_and_ecosystems() {
        let cases = [
            ("/h/.npm/_cacache", Some(Brand::Npm)),
            ("/h/.local/share/pnpm/store", Some(Brand::Pnpm)),
            ("/h/.cache/yarn", Some(Brand::Yarn)),
            ("/h/.bun/install/cache", Some(Brand::Bun)),
            ("/h/.cache/pip", Some(Brand::Python)),
            ("/h/.cache/uv", Some(Brand::Uv)),
            ("/h/.cargo/registry", Some(Brand::Rust)),
            ("/h/.cache/go-build", Some(Brand::Go)),
            ("/h/.cache/ms-playwright", Some(Brand::Playwright)),
            ("/h/.cache/puppeteer", None),
        ];
        for (p, want) in cases {
            assert_eq!(Brand::for_tool_path(Path::new(p)), want, "{p}");
        }
        for (eco, b) in [
            (Ecosystem::Node, Brand::Npm),
            (Ecosystem::Python, Brand::Python),
            (Ecosystem::Rust, Brand::Rust),
            (Ecosystem::Go, Brand::Go),
            (Ecosystem::Browsers, Brand::Playwright),
        ] {
            assert_eq!(Brand::for_ecosystem(eco), Some(b));
        }
        let mut h = Husk::bare(HuskKind::Toolchain, "/h/.cache/uv", 1);
        assert_eq!(Brand::for_husk(&h), Some(Brand::Uv));
        h.agent = Some(AgentKind::Codex);
        assert_eq!(Brand::for_husk(&h), Some(Brand::OpenAi));
        let mut nm = Husk::bare(HuskKind::Ballast, "/p/node_modules", 1);
        assert_eq!(Brand::for_husk(&nm), None);
        nm.ecosystem = Some(Ecosystem::Node);
        assert_eq!(Brand::for_husk(&nm), Some(Brand::Npm));
    }

    #[test]
    fn kinds_and_wards_map_to_glyphs() {
        for kind in HuskKind::all() {
            let _ = Glyph::for_kind(kind).svg();
        }
        let wards = [
            Ward::Occupied { holders: vec![] },
            Ward::NeighborBusy { holders: vec![] },
            Ward::Dirty { files: 1 },
            Ward::Stranded { commits: 1 },
            Ward::Locked { reason: None },
            Ward::Primary,
            Ward::Secrets,
            Ward::OutsideRoots,
            Ward::Warm { minutes: 1 },
            Ward::NoGit,
            Ward::Orphaned { origin: None },
        ];
        for w in &wards {
            let _ = Glyph::for_ward(w).svg();
        }
        assert_eq!(Glyph::for_ward(&wards[3]), Glyph::Stranded);
    }
}
