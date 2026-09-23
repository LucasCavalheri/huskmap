use std::path::{Path, PathBuf};

use crate::domain::{AgentKind, BallastKind, Ecosystem, HuskKind, Risk};
use crate::git::{GitMarker, find_checkouts, git_marker};
use crate::safety::{file_name_str, is_forbidden_path, is_skip_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuskSeed {
    pub kind: HuskKind,
    pub agent: Option<AgentKind>,
    pub ballast: Option<BallastKind>,
    pub ecosystem: Option<Ecosystem>,
    pub marker: Option<PathBuf>,
    pub risk: Risk,
    pub reclaimable: bool,
    pub notes: Vec<String>,
    pub skip_children: bool,
    /// Worktree slot with no checkout inside.
    pub no_git: bool,
}

impl HuskSeed {
    fn new(kind: HuskKind, risk: Risk, note: &str) -> Self {
        Self {
            kind,
            agent: None,
            ballast: None,
            ecosystem: None,
            marker: None,
            risk,
            reclaimable: true,
            notes: vec![note.to_string()],
            skip_children: true,
            no_git: false,
        }
    }

    fn agent(mut self, agent: Option<AgentKind>) -> Self {
        self.agent = agent;
        self
    }

    fn walk_children(mut self) -> Self {
        self.skip_children = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    Continue,
    Skip,
    Husk(HuskSeed),
}

#[derive(Debug, Clone)]
pub struct ClassifyContext<'a> {
    pub home: &'a Path,
    /// Existing agent homes, e.g. `~/.claude`. Caches and sessions only count inside these.
    pub agent_homes: &'a [(PathBuf, AgentKind)],
}

impl ClassifyContext<'_> {
    /// Agent home strictly containing `path`.
    fn agent_home_of(&self, path: &Path) -> Option<(&Path, AgentKind)> {
        self.agent_homes
            .iter()
            .filter(|(root, _)| path != root && path.starts_with(root))
            .max_by_key(|(root, _)| root.as_os_str().len())
            .map(|(root, agent)| (root.as_path(), *agent))
    }
}

/// Dot-dir agent attribution anywhere in the path (`proj/.claude/worktrees/x`).
pub fn agent_for_path(path: &Path) -> Option<AgentKind> {
    path.components().rev().find_map(|c| {
        c.as_os_str()
            .to_str()
            .and_then(AgentKind::from_path_component)
    })
}

#[derive(Debug, Clone, Copy)]
enum Marker {
    /// A file next to the ballast dir, e.g. `package.json` beside `node_modules`.
    Sibling(&'static str),
    /// A file inside the ballast dir, e.g. `pyvenv.cfg`.
    Inside(&'static str),
}

struct BallastRule {
    names: &'static [&'static str],
    kind: BallastKind,
    markers: &'static [Marker],
}

use Marker::{Inside, Sibling};

const PY_PROJECT: [Marker; 7] = [
    Inside("CACHEDIR.TAG"),
    Sibling("pyproject.toml"),
    Sibling("setup.py"),
    Sibling("setup.cfg"),
    Sibling("requirements.txt"),
    Sibling("tox.ini"),
    Sibling("pytest.ini"),
];

/// Ballast needs a marker. A large `target` without `Cargo.toml` is not ballast.
const BALLAST: &[BallastRule] = &[
    BallastRule {
        names: &["node_modules"],
        kind: BallastKind::NodeModules,
        markers: &[Sibling("package.json")],
    },
    BallastRule {
        names: &["target"],
        kind: BallastKind::RustTarget,
        markers: &[Sibling("Cargo.toml"), Inside("CACHEDIR.TAG")],
    },
    BallastRule {
        names: &[".venv", "venv", "env", ".env-py"],
        kind: BallastKind::Venv,
        markers: &[Inside("pyvenv.cfg")],
    },
    BallastRule {
        names: &[".next"],
        kind: BallastKind::Next,
        markers: &[
            Sibling("next.config.js"),
            Sibling("next.config.mjs"),
            Sibling("next.config.ts"),
            Sibling("next.config.cjs"),
            Sibling("package.json"),
        ],
    },
    BallastRule {
        names: &[".turbo"],
        kind: BallastKind::Turbo,
        markers: &[Sibling("turbo.json"), Sibling("package.json")],
    },
    BallastRule {
        names: &[".nuxt", ".output"],
        kind: BallastKind::Nuxt,
        markers: &[Sibling("nuxt.config.ts"), Sibling("nuxt.config.js")],
    },
    BallastRule {
        names: &[".svelte-kit"],
        kind: BallastKind::SvelteKit,
        markers: &[
            Sibling("svelte.config.js"),
            Sibling("svelte.config.ts"),
            Sibling("package.json"),
        ],
    },
    BallastRule {
        names: &[".parcel-cache"],
        kind: BallastKind::Parcel,
        markers: &[Sibling("package.json")],
    },
    BallastRule {
        names: &[".mypy_cache", ".pytest_cache", ".ruff_cache"],
        kind: BallastKind::PyTooling,
        markers: &PY_PROJECT,
    },
    BallastRule {
        names: &[".tox", ".nox"],
        kind: BallastKind::Tox,
        markers: &[
            Sibling("tox.ini"),
            Sibling("noxfile.py"),
            Sibling("pyproject.toml"),
        ],
    },
];

fn ballast_match(path: &Path, name: &str) -> Option<(BallastKind, PathBuf)> {
    let rule = BALLAST.iter().find(|r| r.names.contains(&name))?;
    rule.markers.iter().find_map(|m| {
        let file = match m {
            Sibling(f) => path.parent()?.join(f),
            Inside(f) => path.join(f),
        };
        file.is_file().then_some((rule.kind, file))
    })
}

/// Dirs under an agent home whose children are one session each.
const SESSION_CONTAINERS: &[&str] = &[
    "projects",
    "sessions",
    "chats",
    "file-history",
    "session-env",
    "conversations",
];
/// Dirs under an agent home that are one afterimage as a whole.
const WHOLE_AFTERIMAGES: &[&str] = &["archived_sessions", "todos"];
/// Regenerable dirs under an agent home.
const CACHE_NAMES: &[&str] = &[
    "cache",
    "caches",
    "cacheddata",
    "gpucache",
    "code cache",
    "tmp",
    "debug",
    "logs",
    "log",
    "paste-cache",
    "shell-snapshots",
    "statsig",
    "telemetry",
];

fn is_cache_name(name: &str) -> bool {
    CACHE_NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

fn is_digits(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit())
}

/// `worktrees` dir owned by an agent: `~/.codex/worktrees` or `proj/.claude/worktrees`.
fn is_worktrees_container(dir: &Path, ctx: &ClassifyContext<'_>) -> bool {
    if file_name_str(dir) != Some("worktrees") {
        return false;
    }
    let Some(parent) = dir.parent() else {
        return false;
    };
    file_name_str(parent).is_some_and(|n| AgentKind::from_path_component(n).is_some())
        || ctx.agent_homes.iter().any(|(root, _)| root == parent)
}

/// Nearest agent `worktrees` container among the ancestors, and how deep below it `path` is.
fn worktrees_container_of(path: &Path, ctx: &ClassifyContext<'_>) -> Option<(PathBuf, usize)> {
    path.ancestors()
        .skip(1)
        .enumerate()
        .find(|(_, a)| is_worktrees_container(a, ctx))
        .map(|(depth, a)| (a.to_path_buf(), depth + 1))
}

fn agent_session_rule(
    path: &Path,
    name: &str,
    is_file: bool,
    root: &Path,
    agent: AgentKind,
) -> Option<Classification> {
    let deck = crate::copy::get();
    let rel: Vec<&str> = path
        .strip_prefix(root)
        .ok()?
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let afterimage = || {
        Classification::Husk(
            HuskSeed::new(HuskKind::Afterimage, Risk::Caution, deck.note_afterimage)
                .agent(Some(agent)),
        )
    };

    // Claude's live-session registry. Tiny, and deleting it confuses running sessions.
    if agent == AgentKind::Claude && rel.first() == Some(&"sessions") {
        return Some(Classification::Skip);
    }
    if rel.len() == 1 && WHOLE_AFTERIMAGES.contains(&name) {
        return Some(afterimage());
    }
    let container = rel.iter().position(|c| SESSION_CONTAINERS.contains(c))?;
    let below = &rel[container + 1..];
    // Codex shards sessions by date: sessions/2026/09/23/rollout.jsonl. One husk per day.
    let dated = below.iter().take_while(|c| is_digits(c)).count();
    match below.len() {
        0 => Some(Classification::Continue),
        n if dated == n && n < 3 && !is_file => Some(Classification::Continue),
        n if n == 1 || (dated == 3 && n == 3) => Some(afterimage()),
        _ => None,
    }
}

fn debris_file(path: &Path, name: &str, in_agent_home: Option<AgentKind>) -> Option<HuskSeed> {
    let deck = crate::copy::get();
    if name.starts_with(".aider.") && (name.contains("history")) {
        return Some(
            HuskSeed::new(HuskKind::Debris, Risk::Caution, deck.note_history)
                .agent(Some(AgentKind::Aider)),
        );
    }
    let agent = in_agent_home?;
    if name.ends_with(".log") {
        return Some(
            HuskSeed::new(HuskKind::Debris, Risk::Safe, deck.note_debris).agent(Some(agent)),
        );
    }
    if name == "history.jsonl" || name.ends_with(".chat.history") {
        return Some(
            HuskSeed::new(HuskKind::Debris, Risk::Caution, deck.note_history).agent(Some(agent)),
        );
    }
    let _ = path;
    None
}

/// Classify a path. Marker rules are mandatory. Agent-home rules never apply inside a worktree.
pub fn classify_path(path: &Path, ctx: &ClassifyContext<'_>) -> Classification {
    let deck = crate::copy::get();
    let Some(name) = file_name_str(path) else {
        return Classification::Continue;
    };
    if is_skip_dir(path) || is_forbidden_path(path) {
        return Classification::Skip;
    }

    let worktrees = worktrees_container_of(path, ctx);
    let agent_home = if worktrees.is_none() {
        ctx.agent_home_of(path)
    } else {
        None
    };
    let is_file = path.is_file();

    if is_file {
        if let Some(seed) = debris_file(path, name, agent_home.map(|(_, a)| a)) {
            return Classification::Husk(seed);
        }
        if let Some((root, agent)) = agent_home
            && let Some(Classification::Husk(seed)) =
                agent_session_rule(path, name, true, root, agent)
        {
            return Classification::Husk(seed);
        }
        return Classification::Continue;
    }

    if let Some((kind, marker)) = ballast_match(path, name) {
        let mut seed = HuskSeed::new(
            HuskKind::Ballast,
            Risk::Safe,
            &deck.note_with_marker.replace("{kind}", kind.as_str()),
        )
        .agent(agent_for_path(path));
        seed.ballast = Some(kind);
        seed.ecosystem = Some(kind.ecosystem());
        seed.marker = Some(marker);
        return Classification::Husk(seed);
    }

    if name == ".aider.tags.cache.v3" {
        return Classification::Husk(
            HuskSeed::new(HuskKind::Cache, Risk::Safe, deck.note_cache)
                .agent(Some(AgentKind::Aider)),
        );
    }

    if let Some((container, depth)) = worktrees {
        return worktree_slot(path, &container, depth);
    }

    if let Some((root, agent)) = agent_home {
        if is_worktrees_container(path, ctx) {
            return Classification::Continue;
        }
        if let Some(c) = agent_session_rule(path, name, false, root, agent) {
            return c;
        }
        if is_cache_name(name) {
            return Classification::Husk(
                HuskSeed::new(HuskKind::Cache, Risk::Safe, deck.note_cache).agent(Some(agent)),
            );
        }
        return Classification::Continue;
    }

    match git_marker(path) {
        GitMarker::Linked => Classification::Husk(
            HuskSeed::new(HuskKind::Worktree, Risk::Caution, deck.note_git_worktree)
                .agent(agent_for_path(path))
                .walk_children(),
        ),
        GitMarker::Primary if name.starts_with("agent-") => Classification::Husk(
            HuskSeed::new(HuskKind::Worktree, Risk::Caution, deck.note_agent_clone).walk_children(),
        ),
        _ => Classification::Continue,
    }
}

/// Inside an agent `worktrees` dir. Checkouts are worktrees; an empty slot is a husk of its own.
fn worktree_slot(path: &Path, container: &Path, depth: usize) -> Classification {
    let deck = crate::copy::get();
    let agent = agent_for_path(container);
    match git_marker(path) {
        GitMarker::Linked | GitMarker::Primary => {
            return Classification::Husk(
                HuskSeed::new(HuskKind::Worktree, Risk::Caution, deck.note_worktree_root)
                    .agent(agent)
                    .walk_children(),
            );
        }
        GitMarker::Submodule | GitMarker::None => {}
    }
    if depth == 1 && find_checkouts(path, 3).is_empty() {
        let mut seed =
            HuskSeed::new(HuskKind::Worktree, Risk::Caution, deck.note_empty_slot).agent(agent);
        seed.no_git = true;
        return Classification::Husk(seed);
    }
    Classification::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Fx {
        _tmp: tempfile::TempDir,
        home: PathBuf,
        homes: Vec<(PathBuf, AgentKind)>,
    }

    impl Fx {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().to_path_buf();
            let homes = vec![
                (home.join(".claude"), AgentKind::Claude),
                (home.join(".codex"), AgentKind::Codex),
                (home.join(".cursor"), AgentKind::Cursor),
            ];
            Self {
                _tmp: tmp,
                home,
                homes,
            }
        }

        fn at(&self, rel: &str) -> PathBuf {
            self.home.join(rel)
        }

        fn dir(&self, rel: &str) -> PathBuf {
            let p = self.at(rel);
            fs::create_dir_all(&p).unwrap();
            p
        }

        fn file(&self, rel: &str, body: &str) -> PathBuf {
            let p = self.at(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, body).unwrap();
            p
        }

        fn classify(&self, p: &Path) -> Classification {
            classify_path(
                p,
                &ClassifyContext {
                    home: &self.home,
                    agent_homes: &self.homes,
                },
            )
        }

        fn seed(&self, p: &Path) -> HuskSeed {
            match self.classify(p) {
                Classification::Husk(s) => s,
                other => panic!("{}: {other:?}", p.display()),
            }
        }
    }

    #[test]
    fn ballast_table_requires_markers() {
        let fx = Fx::new();
        type Case<'a> = (&'a str, &'a [(&'a str, bool)], BallastKind);
        let cases: &[Case] = &[
            (
                "a/node_modules",
                &[("a/package.json", false)],
                BallastKind::NodeModules,
            ),
            (
                "b/target",
                &[("b/Cargo.toml", false)],
                BallastKind::RustTarget,
            ),
            (
                "c/target",
                &[("c/target/CACHEDIR.TAG", true)],
                BallastKind::RustTarget,
            ),
            (
                "d/.venv",
                &[("d/.venv/pyvenv.cfg", true)],
                BallastKind::Venv,
            ),
            ("e/.next", &[("e/package.json", false)], BallastKind::Next),
            ("f/.turbo", &[("f/turbo.json", false)], BallastKind::Turbo),
            ("g/.nuxt", &[("g/nuxt.config.ts", false)], BallastKind::Nuxt),
            (
                "h/.svelte-kit",
                &[("h/svelte.config.js", false)],
                BallastKind::SvelteKit,
            ),
            (
                "i/.parcel-cache",
                &[("i/package.json", false)],
                BallastKind::Parcel,
            ),
            (
                "j/.pytest_cache",
                &[("j/pyproject.toml", false)],
                BallastKind::PyTooling,
            ),
            (
                "k/.ruff_cache",
                &[("k/.ruff_cache/CACHEDIR.TAG", true)],
                BallastKind::PyTooling,
            ),
            ("l/.tox", &[("l/tox.ini", false)], BallastKind::Tox),
        ];
        for (dir, markers, kind) in cases {
            let p = fx.dir(dir);
            assert_eq!(fx.classify(&p), Classification::Continue, "{dir} unmarked");
            for (m, _) in *markers {
                fx.file(m, "");
            }
            let seed = fx.seed(&p);
            assert_eq!(seed.kind, HuskKind::Ballast);
            assert_eq!(seed.ballast, Some(*kind), "{dir}");
            assert_eq!(seed.ecosystem, Some(kind.ecosystem()));
            assert_eq!(seed.risk, Risk::Safe);
            assert!(seed.skip_children);
            assert!(seed.marker.is_some());
        }
    }

    #[test]
    fn secrets_and_git_are_skipped() {
        let fx = Fx::new();
        let env = fx.file("p/.env", "SECRET=1");
        assert_eq!(fx.classify(&env), Classification::Skip);
        let git = fx.dir("p/.git");
        assert_eq!(fx.classify(&git), Classification::Skip);
        assert_eq!(fx.classify(Path::new("/")), Classification::Continue);
    }

    #[test]
    fn project_named_like_an_agent_is_not_an_agent_home() {
        let fx = Fx::new();
        let logs = fx.dir("dev/codex/logs");
        assert_eq!(fx.classify(&logs), Classification::Continue);
        let log = fx.file("dev/claude/debug.log", "x");
        assert_eq!(fx.classify(&log), Classification::Continue);
    }

    #[test]
    fn agent_home_caches_and_debris() {
        let fx = Fx::new();
        for dir in [
            ".cursor/Cache",
            ".claude/debug",
            ".codex/cache",
            ".claude/shell-snapshots",
        ] {
            let seed = fx.seed(&fx.dir(dir));
            assert_eq!(seed.kind, HuskKind::Cache, "{dir}");
        }
        assert_eq!(
            fx.seed(&fx.dir(".codex/cache")).agent,
            Some(AgentKind::Codex)
        );
        let log = fx.seed(&fx.file(".codex/log/x.log", "x"));
        assert_eq!((log.kind, log.risk), (HuskKind::Debris, Risk::Safe));
        let hist = fx.seed(&fx.file(".claude/history.jsonl", "x"));
        assert_eq!((hist.kind, hist.risk), (HuskKind::Debris, Risk::Caution));
        let chat = fx.seed(&fx.file(".codex/a.chat.history", "x"));
        assert_eq!(chat.kind, HuskKind::Debris);
        let plain = fx.file(".claude/settings.json", "{}");
        assert_eq!(fx.classify(&plain), Classification::Continue);
        let other = fx.dir(".claude/agents");
        assert_eq!(fx.classify(&other), Classification::Continue);
    }

    #[test]
    fn aider_leftovers_in_projects() {
        let fx = Fx::new();
        let hist = fx.seed(&fx.file("dev/p/.aider.chat.history.md", "x"));
        assert_eq!(hist.kind, HuskKind::Debris);
        assert_eq!(hist.agent, Some(AgentKind::Aider));
        let tags = fx.seed(&fx.dir("dev/p/.aider.tags.cache.v3"));
        assert_eq!(tags.kind, HuskKind::Cache);
        let conf = fx.file("dev/p/.aider.conf.yml", "x");
        assert_eq!(fx.classify(&conf), Classification::Continue);
    }

    #[test]
    fn sessions_one_husk_per_project_or_day() {
        let fx = Fx::new();
        let proj = fx.dir(".claude/projects/-home-me-dev-app");
        let seed = fx.seed(&proj);
        assert_eq!(seed.kind, HuskKind::Afterimage);
        assert_eq!(seed.agent, Some(AgentKind::Claude));
        assert_eq!(
            fx.classify(&fx.dir(".claude/projects")),
            Classification::Continue
        );
        assert_eq!(
            fx.classify(&fx.dir(".codex/sessions/2026")),
            Classification::Continue
        );
        assert_eq!(
            fx.classify(&fx.dir(".codex/sessions/2026/09")),
            Classification::Continue
        );
        let day = fx.seed(&fx.dir(".codex/sessions/2026/09/23"));
        assert_eq!(day.kind, HuskKind::Afterimage);
        let stray = fx.file(".codex/sessions/2026/09/stray.jsonl", "x");
        assert_eq!(fx.classify(&stray), Classification::Continue);
        let loose = fx.seed(&fx.file(".codex/sessions/rollout.jsonl", "x"));
        assert_eq!(loose.kind, HuskKind::Afterimage);
        let archived = fx.seed(&fx.dir(".codex/archived_sessions"));
        assert_eq!(archived.kind, HuskKind::Afterimage);
        let todos = fx.seed(&fx.dir(".claude/todos"));
        assert_eq!(todos.kind, HuskKind::Afterimage);
        let hist = fx.seed(&fx.dir(".claude/file-history/abc"));
        assert_eq!(hist.kind, HuskKind::Afterimage);
        let deep = fx.dir(".claude/file-history/abc/deeper");
        assert_eq!(fx.classify(&deep), Classification::Continue);
    }

    #[test]
    fn claude_session_registry_is_skipped() {
        let fx = Fx::new();
        assert_eq!(
            fx.classify(&fx.dir(".claude/sessions")),
            Classification::Skip
        );
        // the walker never descends, but a file handed in directly is still not a husk
        let pid = fx.file(".claude/sessions/1234.json", "{}");
        assert_eq!(fx.classify(&pid), Classification::Continue);
    }

    #[test]
    fn linked_worktree_anywhere_and_submodule_ignored() {
        let fx = Fx::new();
        let wt = fx.dir("dev/app-worktrees/fix-bug");
        fx.file(
            "dev/app-worktrees/fix-bug/.git",
            "gitdir: /r/.git/worktrees/fix-bug\n",
        );
        let seed = fx.seed(&wt);
        assert_eq!(seed.kind, HuskKind::Worktree);
        assert!(!seed.skip_children);
        let sub = fx.dir("dev/app/vendor/lib");
        fx.file(
            "dev/app/vendor/lib/.git",
            "gitdir: ../../.git/modules/lib\n",
        );
        assert_eq!(fx.classify(&sub), Classification::Continue);
        let primary = fx.dir("dev/app");
        fx.dir("dev/app/.git");
        assert_eq!(fx.classify(&primary), Classification::Continue);
        let clone = fx.dir("dev/agent-7");
        fx.dir("dev/agent-7/.git");
        assert_eq!(fx.seed(&clone).kind, HuskKind::Worktree);
    }

    #[test]
    fn agent_worktree_slots() {
        let fx = Fx::new();
        // codex: worktrees/<id>/<repo>
        let slot = fx.dir(".codex/worktrees/a1b2");
        let repo = fx.dir(".codex/worktrees/a1b2/app");
        fx.file(
            ".codex/worktrees/a1b2/app/.git",
            "gitdir: /r/.git/worktrees/app\n",
        );
        assert_eq!(
            fx.classify(&fx.at(".codex/worktrees")),
            Classification::Continue
        );
        assert_eq!(fx.classify(&slot), Classification::Continue);
        let seed = fx.seed(&repo);
        assert_eq!(seed.kind, HuskKind::Worktree);
        assert_eq!(seed.agent, Some(AgentKind::Codex));
        // cache-named dirs inside a worktree are project files, not agent caches
        let logs = fx.dir(".codex/worktrees/a1b2/app/logs");
        assert_eq!(fx.classify(&logs), Classification::Continue);
        let nm = fx.dir(".codex/worktrees/a1b2/app/node_modules");
        fx.file(".codex/worktrees/a1b2/app/package.json", "{}");
        assert_eq!(fx.seed(&nm).kind, HuskKind::Ballast);
        // empty slot
        let empty = fx.dir(".codex/worktrees/dead");
        fx.file(".codex/worktrees/dead/leftover.txt", "x");
        let seed = fx.seed(&empty);
        assert!(seed.no_git);
        assert!(seed.skip_children);
        // project-level .claude/worktrees
        let cw = fx.dir("dev/app/.claude/worktrees/feature");
        fx.file(
            "dev/app/.claude/worktrees/feature/.git",
            "gitdir: /x/.git/worktrees/feature",
        );
        let seed = fx.seed(&cw);
        assert_eq!(seed.agent, Some(AgentKind::Claude));
        let sub = fx.dir("dev/app/.claude/worktrees/feature/src");
        assert_eq!(fx.classify(&sub), Classification::Continue);
        // a plain dir named worktrees is not an agent container
        let plain = fx.dir("dev/worktrees/x");
        assert_eq!(fx.classify(&plain), Classification::Continue);
        assert!(!is_worktrees_container(
            Path::new("worktrees"),
            &ClassifyContext {
                home: &fx.home,
                agent_homes: &fx.homes
            }
        ));
    }

    #[test]
    fn agent_for_nested_path() {
        assert_eq!(
            agent_for_path(Path::new("/home/me/.opencode/sessions/1")),
            Some(AgentKind::OpenCode)
        );
        assert_eq!(agent_for_path(Path::new("/home/me/codex/1")), None);
    }

    #[test]
    fn digits() {
        assert!(is_digits("2026"));
        assert!(!is_digits(""));
        assert!(!is_digits("20a6"));
    }
}
