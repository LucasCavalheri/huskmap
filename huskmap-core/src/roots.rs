//! Where to look. Linux only; paths follow the XDG Base Directory spec.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::domain::{AgentKind, Ecosystem};

/// Resolved base directories for one home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub home: PathBuf,
    /// `$XDG_CACHE_HOME`, default `~/.cache`.
    pub cache: PathBuf,
    /// `$XDG_DATA_HOME`, default `~/.local/share`.
    pub data: PathBuf,
    /// `$XDG_CONFIG_HOME`, default `~/.config`.
    pub config: PathBuf,
    /// `$XDG_STATE_HOME`, default `~/.local/state`.
    pub state: PathBuf,
    /// `$CARGO_HOME`, default `~/.cargo`.
    pub cargo: PathBuf,
}

impl Dirs {
    /// Spec defaults, ignoring the environment. Tests use this.
    pub fn for_home(home: impl Into<PathBuf>) -> Self {
        Self::from_env(home, |_| None)
    }

    /// Resolve with `var` lookups. Relative values are invalid per the spec and ignored.
    pub fn from_env(home: impl Into<PathBuf>, var: impl Fn(&str) -> Option<OsString>) -> Self {
        let home = home.into();
        let pick = |key: &str, default: &str| {
            var(key)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| join_relative(&home, default))
        };
        Self {
            cache: pick("XDG_CACHE_HOME", ".cache"),
            data: pick("XDG_DATA_HOME", ".local/share"),
            config: pick("XDG_CONFIG_HOME", ".config"),
            state: pick("XDG_STATE_HOME", ".local/state"),
            cargo: pick("CARGO_HOME", ".cargo"),
            home,
        }
    }

    /// From the real process environment.
    pub fn current(home: impl Into<PathBuf>) -> Self {
        Self::from_env(home, |k| std::env::var_os(k))
    }

    pub fn base(&self, base: Base) -> &Path {
        match base {
            Base::Home => &self.home,
            Base::Cache => &self.cache,
            Base::Data => &self.data,
            Base::Config => &self.config,
            Base::Cargo => &self.cargo,
        }
    }

    /// `$XDG_STATE_HOME/huskmap`.
    pub fn huskmap_state(&self) -> PathBuf {
        self.state.join("huskmap")
    }

    pub fn last_report(&self) -> PathBuf {
        self.huskmap_state().join("last-report.json")
    }
}

/// Which base directory a row is relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Home,
    Cache,
    Data,
    Config,
    Cargo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanRootKind {
    /// Where an agent keeps sessions, caches and worktrees. Example: `~/.codex`.
    AgentHome(AgentKind),
    /// Common project home, walked for worktrees and ballast. Example: `~/dev`.
    ProjectHome,
    /// Package-manager cache, reported whole and never walked. Example: `~/.npm/_cacache`.
    Toolchain(Ecosystem),
    /// Agent-owned cache outside the agent home, reported whole. Example: `$XDG_CACHE_HOME/codex-runtimes`.
    AgentCache(AgentKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanRootSpec {
    pub id: &'static str,
    /// Short name shown to people: `npm`, `claude`, `Documents`.
    pub label: &'static str,
    pub kind: ScanRootKind,
    pub base: Base,
    pub relative: &'static str,
}

const fn agent(id: &'static str, a: AgentKind, base: Base, rel: &'static str) -> ScanRootSpec {
    ScanRootSpec {
        id,
        label: a.label(),
        kind: ScanRootKind::AgentHome(a),
        base,
        relative: rel,
    }
}

const fn project(rel: &'static str) -> ScanRootSpec {
    ScanRootSpec {
        id: rel,
        label: rel,
        kind: ScanRootKind::ProjectHome,
        base: Base::Home,
        relative: rel,
    }
}

const fn tool(
    id: &'static str,
    label: &'static str,
    eco: Ecosystem,
    base: Base,
    rel: &'static str,
) -> ScanRootSpec {
    ScanRootSpec {
        id,
        label,
        kind: ScanRootKind::Toolchain(eco),
        base,
        relative: rel,
    }
}

const fn agent_cache(id: &'static str, a: AgentKind, rel: &'static str) -> ScanRootSpec {
    ScanRootSpec {
        id,
        label: a.label(),
        kind: ScanRootKind::AgentCache(a),
        base: Base::Cache,
        relative: rel,
    }
}

impl AgentKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::OpenCode => "opencode",
            Self::Gemini => "gemini",
            Self::Aider => "aider",
            Self::Grok => "grok",
            Self::Unknown => "unknown",
        }
    }
}

use AgentKind as A;
use Base::{Cache, Cargo, Config, Data, Home};
use Ecosystem as E;

/// Single table of known scan roots. Extend here, not with scattered `if`s.
pub const SCAN_ROOTS: &[ScanRootSpec] = &[
    agent("claude", A::Claude, Home, ".claude"),
    agent("codex", A::Codex, Home, ".codex"),
    agent("cursor", A::Cursor, Home, ".cursor"),
    agent("cursor-config", A::Cursor, Config, "Cursor"),
    agent("opencode", A::OpenCode, Home, ".opencode"),
    agent("opencode-data", A::OpenCode, Data, "opencode"),
    agent("gemini", A::Gemini, Home, ".gemini"),
    agent("aider", A::Aider, Home, ".aider"),
    agent("grok", A::Grok, Home, ".grok"),
    project("dev"),
    project("src"),
    project("code"),
    project("Code"),
    project("projects"),
    project("Projects"),
    project("Developer"),
    project("workspace"),
    project("repos"),
    project("git"),
    project("work"),
    project("Documents"),
    project("Documentos"),
    tool("npm", "npm", E::Node, Home, ".npm/_cacache"),
    tool("pnpm", "pnpm", E::Node, Data, "pnpm/store"),
    tool("pnpm-home", "pnpm", E::Node, Home, ".pnpm-store"),
    tool("yarn", "yarn", E::Node, Cache, "yarn"),
    tool("yarn-berry", "yarn", E::Node, Home, ".yarn/berry/cache"),
    tool("bun", "bun", E::Node, Home, ".bun/install/cache"),
    tool("pip", "pip", E::Python, Cache, "pip"),
    tool("uv", "uv", E::Python, Cache, "uv"),
    tool("poetry", "poetry", E::Python, Cache, "pypoetry"),
    tool("cargo-registry", "cargo", E::Rust, Cargo, "registry"),
    tool("cargo-git", "cargo git", E::Rust, Cargo, "git"),
    tool("go-build", "go build", E::Go, Cache, "go-build"),
    tool(
        "playwright",
        "playwright",
        E::Browsers,
        Cache,
        "ms-playwright",
    ),
    tool("puppeteer", "puppeteer", E::Browsers, Cache, "puppeteer"),
    agent_cache("codex-runtimes", A::Codex, "codex-runtimes"),
    agent_cache("opencode-cache", A::OpenCode, "opencode"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub spec_id: &'static str,
    pub label: &'static str,
    pub kind: ScanRootKind,
    pub path: PathBuf,
    pub exists: bool,
}

fn join_relative(base: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(base.to_path_buf(), |p, c| p.join(c))
}

pub fn resolve_known_roots(dirs: &Dirs) -> Vec<ResolvedRoot> {
    SCAN_ROOTS
        .iter()
        .map(|spec| {
            let path = join_relative(dirs.base(spec.base), spec.relative);
            let exists = path.is_dir();
            ResolvedRoot {
                spec_id: spec.id,
                label: spec.label,
                kind: spec.kind,
                path,
                exists,
            }
        })
        .collect()
}

/// Existing roots, with any root nested inside another dropped so nothing is walked twice.
pub fn existing_roots(dirs: &Dirs) -> Vec<ResolvedRoot> {
    let found: Vec<ResolvedRoot> = resolve_known_roots(dirs)
        .into_iter()
        .filter(|r| r.exists)
        .collect();
    found
        .iter()
        .filter(|r| {
            !found
                .iter()
                .any(|o| o.path != r.path && r.path.starts_with(&o.path))
        })
        .cloned()
        .collect()
}

/// Agent homes, existing or not, for attributing caches and sessions.
pub fn agent_homes(dirs: &Dirs) -> Vec<(PathBuf, AgentKind)> {
    resolve_known_roots(dirs)
        .into_iter()
        .filter_map(|r| match r.kind {
            ScanRootKind::AgentHome(agent) => Some((r.path, agent)),
            _ => None,
        })
        .collect()
}

/// Drop paths nested inside another path in the list. Order is preserved.
pub fn dedupe_nested(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in &paths {
        let covered = paths.iter().any(|o| o != p && p.starts_with(o));
        if !covered && !out.contains(p) {
            out.push(p.clone());
        }
    }
    out
}

/// X11 or Wayland.
pub fn display_available_from(has_var: impl Fn(&str) -> bool) -> bool {
    has_var("WAYLAND_DISPLAY") || has_var("DISPLAY")
}

pub fn display_available() -> bool {
    display_available_from(|key| std::env::var_os(key).is_some_and(|v| !v.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_ids_are_unique_and_required_present() {
        let ids: Vec<_> = SCAN_ROOTS.iter().map(|s| s.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate id");
        for needed in [
            "codex",
            "claude",
            "cursor",
            "cursor-config",
            "opencode",
            "gemini",
            "aider",
            "grok",
            "dev",
            "src",
            "projects",
            "Developer",
            "workspace",
            "Documentos",
            "npm",
            "pip",
            "uv",
            "cargo-registry",
        ] {
            assert!(ids.contains(&needed), "missing {needed}");
        }
    }

    #[test]
    fn xdg_defaults_and_overrides() {
        let d = Dirs::for_home("/h");
        assert_eq!(d.cache, Path::new("/h/.cache"));
        assert_eq!(d.data, Path::new("/h/.local/share"));
        assert_eq!(d.config, Path::new("/h/.config"));
        assert_eq!(d.state, Path::new("/h/.local/state"));
        assert_eq!(d.cargo, Path::new("/h/.cargo"));
        assert_eq!(
            d.last_report(),
            Path::new("/h/.local/state/huskmap/last-report.json")
        );
        let env = |k: &str| match k {
            "XDG_CACHE_HOME" => Some(OsString::from("/var/cache/me")),
            "XDG_DATA_HOME" => Some(OsString::from("relative/is/ignored")),
            "CARGO_HOME" => Some(OsString::from("/opt/cargo")),
            _ => None,
        };
        let d = Dirs::from_env("/h", env);
        assert_eq!(d.cache, Path::new("/var/cache/me"));
        assert_eq!(d.data, Path::new("/h/.local/share"));
        assert_eq!(d.cargo, Path::new("/opt/cargo"));
        let r = resolve_known_roots(&d);
        let path = |id: &str| r.iter().find(|x| x.spec_id == id).unwrap().path.clone();
        assert_eq!(path("uv"), Path::new("/var/cache/me/uv"));
        assert_eq!(path("cargo-registry"), Path::new("/opt/cargo/registry"));
        assert_eq!(path("cursor-config"), Path::new("/h/.config/Cursor"));
        assert_eq!(path("npm"), Path::new("/h/.npm/_cacache"));
        let _ = Dirs::current("/h");
    }

    #[test]
    fn row_builders_at_runtime() {
        assert_eq!(agent("x", A::Grok, Home, ".x").label, "grok");
        assert_eq!(project("dev").kind, ScanRootKind::ProjectHome);
        assert_eq!(tool("t", "t", E::Go, Cache, "t").base, Cache);
        assert_eq!(
            agent_cache("c", A::Codex, "c").kind,
            ScanRootKind::AgentCache(A::Codex)
        );
        for a in AgentKind::KNOWN {
            assert_eq!(a.label(), a.as_str());
        }
    }

    #[test]
    fn labels_follow_kind() {
        for spec in SCAN_ROOTS {
            match spec.kind {
                ScanRootKind::AgentHome(a) | ScanRootKind::AgentCache(a) => {
                    assert_eq!(spec.label, a.as_str())
                }
                ScanRootKind::ProjectHome => assert_eq!(spec.label, spec.relative),
                ScanRootKind::Toolchain(_) => assert!(!spec.label.is_empty()),
            }
        }
        assert_eq!(AgentKind::Unknown.label(), "unknown");
    }

    #[test]
    fn existing_roots_skip_missing_and_nested() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        std::fs::create_dir_all(tmp.path().join("Documents/code")).unwrap();
        std::fs::write(tmp.path().join("dev"), "not a dir").unwrap();
        let dirs = Dirs::for_home(tmp.path());
        let paths: Vec<_> = existing_roots(&dirs)
            .iter()
            .map(|r| r.path.clone())
            .collect();
        assert_eq!(
            paths,
            vec![tmp.path().join(".claude"), tmp.path().join("Documents")]
        );
        let homes = agent_homes(&dirs);
        assert!(
            homes
                .iter()
                .any(|(p, a)| p.ends_with(".codex") && *a == AgentKind::Codex)
        );
        // a project home inside another root is walked once
        let mut xdg = Dirs::for_home(tmp.path());
        xdg.cache = tmp.path().join("Documents/code/cache");
        std::fs::create_dir_all(xdg.cache.join("uv")).unwrap();
        assert!(existing_roots(&xdg).iter().all(|r| r.spec_id != "uv"));
    }

    #[test]
    fn dedupe_nested_paths() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(
            dedupe_nested(vec![p("/a/b"), p("/a"), p("/c"), p("/c"), p("/ab")]),
            vec![p("/a"), p("/c"), p("/ab")]
        );
    }

    #[test]
    fn display_detection() {
        assert!(!display_available_from(|_| false));
        assert!(display_available_from(|k| k == "DISPLAY"));
        assert!(display_available_from(|k| k == "WAYLAND_DISPLAY"));
        let _ = display_available();
    }
}
