//! The filter language behind the search box and the filter chips.
//!
//! `tipo:worktree peso:>500mb idade:>30d -node_modules "duas palavras"`. Every term must match.
//! Keys and values are accepted in English and Portuguese, whatever the interface language.
//! Chips edit the same text, so what you click is what you could have typed.

use huskmap_core::{AgentKind, Husk, HuskKind, Locale, Millis};

use crate::view_model::Tone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Free,
    Caution,
    Guarded,
    Untouchable,
    Occupied,
    Dirty,
    Stranded,
    Secrets,
    Orphaned,
    Marked,
}

impl Status {
    pub const ALL: [Status; 10] = [
        Status::Free,
        Status::Caution,
        Status::Guarded,
        Status::Untouchable,
        Status::Occupied,
        Status::Dirty,
        Status::Stranded,
        Status::Secrets,
        Status::Orphaned,
        Status::Marked,
    ];

    /// The value a chip writes, in the interface language.
    pub fn token(self, locale: Locale) -> &'static str {
        let (en, pt) = match self {
            Self::Free => ("removable", "livre"),
            Self::Caution => ("caution", "atencao"),
            Self::Guarded => ("protected", "protegido"),
            Self::Untouchable => ("blocked", "bloqueado"),
            Self::Occupied => ("inuse", "emuso"),
            Self::Dirty => ("uncommitted", "semcommit"),
            Self::Stranded => ("unpushed", "sempush"),
            Self::Secrets => ("secrets", "segredos"),
            Self::Orphaned => ("orphan", "orfao"),
            Self::Marked => ("marked", "marcado"),
        };
        if locale == Locale::PtBr { pt } else { en }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match fold(raw).as_str() {
            "removable" | "free" | "safe" | "livre" | "pode" | "podeapagar" => Self::Free,
            "caution" | "check" | "atencao" | "cuidado" => Self::Caution,
            "protected" | "guarded" | "protegido" => Self::Guarded,
            "blocked" | "untouchable" | "bloqueado" | "intocavel" => Self::Untouchable,
            "inuse" | "busy" | "occupied" | "emuso" | "ocupado" => Self::Occupied,
            "uncommitted" | "dirty" | "semcommit" | "alterado" => Self::Dirty,
            "unpushed" | "stranded" | "sempush" => Self::Stranded,
            "secrets" | "secret" | "segredos" | "segredo" => Self::Secrets,
            "orphan" | "orphaned" | "gone" | "orfao" | "orfa" => Self::Orphaned,
            "marked" | "marcado" | "marcados" => Self::Marked,
            _ => return None,
        })
    }
}

pub fn kind_token(kind: HuskKind, locale: Locale) -> &'static str {
    let (en, pt) = match kind {
        HuskKind::Worktree => ("worktree", "worktree"),
        HuskKind::Ballast => ("deps", "deps"),
        HuskKind::Toolchain => ("packages", "pacotes"),
        HuskKind::Afterimage => ("sessions", "sessoes"),
        HuskKind::Cache => ("caches", "caches"),
        HuskKind::Debris => ("logs", "logs"),
    };
    if locale == Locale::PtBr { pt } else { en }
}

pub fn parse_kind(raw: &str) -> Option<HuskKind> {
    Some(match fold(raw).as_str() {
        "worktree" | "worktrees" | "wt" => HuskKind::Worktree,
        "deps" | "dep" | "dependencies" | "dependency" | "dependencias" | "dependencia"
        | "ballast" => HuskKind::Ballast,
        "packages" | "package" | "pkg" | "pacotes" | "pacote" | "toolchain" | "toolchains" => {
            HuskKind::Toolchain
        }
        "sessions" | "session" | "sessoes" | "sessao" | "afterimage" | "afterimages" => {
            HuskKind::Afterimage
        }
        "caches" | "cache" => HuskKind::Cache,
        "logs" | "log" | "debris" => HuskKind::Debris,
        _ => return None,
    })
}

/// Lowercase, accents off: `Sessões` → `sessoes`.
fn fold(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
}

impl Cmp {
    fn split(raw: &str) -> (Self, &str) {
        for (op, cmp) in [
            (">=", Self::Ge),
            ("<=", Self::Le),
            (">", Self::Gt),
            ("<", Self::Lt),
            ("=", Self::Eq),
        ] {
            if let Some(rest) = raw.strip_prefix(op) {
                return (cmp, rest);
            }
        }
        (Self::Ge, raw)
    }

    fn test(self, value: u64, bound: u64) -> bool {
        match self {
            Self::Gt => value > bound,
            Self::Ge => value >= bound,
            Self::Lt => value < bound,
            Self::Le => value <= bound,
            Self::Eq => value == bound,
        }
    }
}

/// Split `12.5gb` into the number and its unit.
fn number_unit(raw: &str) -> Option<(f64, String)> {
    let raw = raw.trim().replace(',', ".");
    let at = raw
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(raw.len());
    let n: f64 = raw[..at].parse().ok()?;
    (n >= 0.0).then(|| (n, fold(&raw[at..])))
}

/// `>500mb` → (Gt, bytes). A bare number is megabytes.
pub fn parse_size(raw: &str) -> Option<(Cmp, u64)> {
    let (cmp, rest) = Cmp::split(raw.trim());
    let (n, unit) = number_unit(rest)?;
    let scale: f64 = match unit.as_str() {
        "b" => 1.0,
        "k" | "kb" => 1024.0,
        "" | "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((cmp, (n * scale) as u64))
}

/// `>30d` → (Gt, ms). A bare number is days. `m` is months, `min` minutes.
pub fn parse_age(raw: &str) -> Option<(Cmp, Millis)> {
    const MIN: f64 = 60_000.0;
    const DAY: f64 = 86_400_000.0;
    let (cmp, rest) = Cmp::split(raw.trim());
    let (n, unit) = number_unit(rest)?;
    let scale = match unit.as_str() {
        "min" | "mins" => MIN,
        "h" | "hr" | "hrs" | "hora" | "horas" => 60.0 * MIN,
        "" | "d" | "day" | "days" | "dia" | "dias" => DAY,
        "w" | "wk" | "week" | "weeks" | "s" | "sem" | "semana" | "semanas" => 7.0 * DAY,
        "m" | "mo" | "month" | "months" | "mes" | "meses" => 30.0 * DAY,
        "y" | "yr" | "year" | "years" | "a" | "ano" | "anos" => 365.0 * DAY,
        _ => return None,
    };
    Some((cmp, (n * scale) as Millis))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Kind,
    Status,
    Agent,
    Size,
    Age,
    Branch,
    Path,
    Eco,
}

impl Key {
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match fold(raw).as_str() {
            "kind" | "type" | "tipo" => Self::Kind,
            "is" | "status" | "e" | "situacao" => Self::Status,
            "agent" | "agente" => Self::Agent,
            "size" | "peso" | "tamanho" => Self::Size,
            "age" | "idade" => Self::Age,
            "branch" => Self::Branch,
            "path" | "caminho" => Self::Path,
            "eco" | "ecosystem" => Self::Eco,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Kind(Vec<HuskKind>),
    Status(Vec<Status>),
    Agent(Vec<String>),
    Size(Cmp, u64),
    Age(Cmp, Millis),
    Branch(String),
    Path(String),
    Eco(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub negated: bool,
    pub field: Field,
}

/// What a term needs to know about a husk beyond the husk itself.
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    pub tone: Tone,
    pub marked: bool,
    pub now: Millis,
    /// Display name, so what you see is what you can search.
    pub name: &'a str,
}

/// Whitespace-separated tokens; double quotes keep words together and are kept in the token.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unquote(raw: &str) -> String {
    raw.replace('"', "")
}

/// `-kind:deps` → (negated, Some(Kind), "deps"). Unknown keys are plain text.
fn split_token(token: &str) -> (bool, Option<Key>, String) {
    let (negated, body) = match token.strip_prefix('-') {
        Some(rest) if !rest.is_empty() => (true, rest),
        _ => (false, token),
    };
    if let Some((k, v)) = body.split_once(':')
        && !k.starts_with('"')
        && let Some(key) = Key::parse(k)
    {
        return (negated, Some(key), unquote(v));
    }
    (negated, None, unquote(body))
}

fn list<T>(raw: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    let v: Vec<T> = raw
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(parse)
        .collect::<Option<_>>()?;
    (!v.is_empty()).then_some(v)
}

pub fn parse_term(token: &str) -> Option<Term> {
    let (negated, key, value) = split_token(token);
    let lower = value.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    let field = match key {
        Some(Key::Kind) => list(&value, parse_kind).map(Field::Kind),
        Some(Key::Status) => list(&value, Status::parse).map(Field::Status),
        Some(Key::Agent) => list(&value, |s| Some(fold(s))).map(Field::Agent),
        Some(Key::Size) => parse_size(&value).map(|(c, b)| Field::Size(c, b)),
        Some(Key::Age) => parse_age(&value).map(|(c, a)| Field::Age(c, a)),
        Some(Key::Branch) => Some(Field::Branch(lower.clone())),
        Some(Key::Path) => Some(Field::Path(lower.clone())),
        Some(Key::Eco) => Some(Field::Eco(fold(&value))),
        None => Some(Field::Text(lower.clone())),
    }
    // A value that does not parse searches as text, so a typo shows "no items" instead of
    // silently widening the list.
    .unwrap_or_else(|| Field::Text(unquote(token).trim_start_matches('-').to_lowercase()));
    Some(Term { negated, field })
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub terms: Vec<Term>,
}

fn status_holds(status: Status, husk: &Husk, facts: &Facts<'_>) -> bool {
    match status {
        Status::Free => facts.tone == Tone::Free,
        Status::Caution => facts.tone == Tone::Caution,
        Status::Guarded => facts.tone == Tone::Guarded,
        Status::Untouchable => facts.tone == Tone::Untouchable,
        Status::Occupied => husk.has_ward("occupied") || husk.has_ward("neighbor_busy"),
        Status::Dirty => husk.has_ward("dirty"),
        Status::Stranded => husk.has_ward("stranded"),
        Status::Secrets => husk.contains_secrets || husk.has_ward("secrets"),
        Status::Orphaned => husk.has_ward("orphaned"),
        Status::Marked => facts.marked,
    }
}

fn agent_names(husk: &Husk) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = husk.agent.map(AgentKind::as_str).into_iter().collect();
    for h in husk.holders() {
        if let Some(a) = h.agent {
            names.push(a.as_str());
        }
    }
    names
}

impl Term {
    fn holds(&self, husk: &Husk, facts: &Facts<'_>) -> bool {
        let yes = match &self.field {
            Field::Kind(kinds) => kinds.contains(&husk.kind),
            Field::Status(list) => list.iter().any(|s| status_holds(*s, husk, facts)),
            Field::Agent(list) => {
                let names = agent_names(husk);
                list.iter()
                    .any(|want| names.iter().any(|n| n.contains(want.as_str())))
            }
            Field::Size(cmp, bytes) => cmp.test(husk.size_bytes, *bytes),
            Field::Age(cmp, ms) => husk.age_ms(facts.now).is_some_and(|a| cmp.test(a, *ms)),
            Field::Branch(text) => husk
                .git
                .as_ref()
                .and_then(|g| g.branch.as_deref())
                .is_some_and(|b| b.to_lowercase().contains(text)),
            Field::Path(text) => {
                husk.path.to_string_lossy().to_lowercase().contains(text)
                    || husk
                        .project
                        .as_ref()
                        .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(text))
            }
            Field::Eco(eco) => husk
                .ecosystem
                .is_some_and(|e| e.as_str().contains(eco.as_str())),
            Field::Text(text) => {
                let branch = husk.git.as_ref().and_then(|g| g.branch.as_deref());
                husk.path.to_string_lossy().to_lowercase().contains(text)
                    || facts.name.to_lowercase().contains(text)
                    || branch.is_some_and(|b| b.to_lowercase().contains(text))
                    || agent_names(husk).iter().any(|n| n.contains(text.as_str()))
                    || husk
                        .project
                        .as_ref()
                        .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(text))
            }
        };
        yes != self.negated
    }
}

impl Query {
    pub fn parse(text: &str) -> Self {
        Self {
            terms: tokens(text).iter().filter_map(|t| parse_term(t)).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn matches(&self, husk: &Husk, facts: &Facts<'_>) -> bool {
        self.terms.iter().all(|t| t.holds(husk, facts))
    }

    /// Kinds a positive `kind:` term narrows to, if any.
    pub fn kinds(&self) -> Option<Vec<HuskKind>> {
        self.terms.iter().find_map(|t| match (&t.field, t.negated) {
            (Field::Kind(k), false) => Some(k.clone()),
            _ => None,
        })
    }
}

// ---- editing the text from chips ----

fn positive_with(token: &str, key: Key) -> bool {
    matches!(split_token(token), (false, Some(k), _) if k == key)
}

fn join(tokens: Vec<String>) -> String {
    tokens.join(" ")
}

/// Is `value` among the values of a positive `key:` token?
pub fn has_value(text: &str, key: Key, value: &str) -> bool {
    tokens(text).iter().any(|t| {
        let (neg, k, v) = split_token(t);
        !neg && k == Some(key) && v.split(',').any(|x| same_value(key, x, value))
    })
}

fn same_value(key: Key, a: &str, b: &str) -> bool {
    match key {
        Key::Kind => parse_kind(a).is_some_and(|k| Some(k) == parse_kind(b)),
        Key::Status => Status::parse(a).is_some_and(|s| Some(s) == Status::parse(b)),
        Key::Size => parse_size(a).is_some() && parse_size(a) == parse_size(b),
        Key::Age => parse_age(a).is_some() && parse_age(a) == parse_age(b),
        _ => fold(a) == fold(b),
    }
}

/// Add `value` to the first positive `key:` token, or take it out if it is already there.
pub fn toggle_value(text: &str, key: Key, value: &str, written_key: &str) -> String {
    let mut toks = tokens(text);
    let Some(i) = toks.iter().position(|t| positive_with(t, key)) else {
        toks.push(format!("{written_key}:{value}"));
        return join(toks);
    };
    let (_, _, values) = split_token(&toks[i]);
    let head = toks[i]
        .split_once(':')
        .map_or(written_key, |(k, _)| k)
        .to_string();
    let mut vals: Vec<String> = values
        .split(',')
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect();
    match vals.iter().position(|v| same_value(key, v, value)) {
        Some(at) => {
            vals.remove(at);
        }
        None => vals.push(value.to_string()),
    }
    if vals.is_empty() {
        toks.remove(i);
    } else {
        toks[i] = format!("{head}:{}", vals.join(","));
    }
    join(toks)
}

/// Replace every positive `key:` token with one `key:value`, or drop them all for `None`.
pub fn set_single(text: &str, key: Key, value: Option<&str>, written_key: &str) -> String {
    let mut toks: Vec<String> = tokens(text)
        .into_iter()
        .filter(|t| !positive_with(t, key))
        .collect();
    if let Some(v) = value {
        toks.push(format!("{written_key}:{v}"));
    }
    join(toks)
}

/// The legend and the 1-6 keys: show only `kind`, or everything again if that is the view.
pub fn only_kind(text: &str, kind: HuskKind, locale: Locale, written_key: &str) -> String {
    let only_this = Query::parse(text).kinds().is_some_and(|k| k == [kind]);
    set_single(
        text,
        Key::Kind,
        (!only_this).then(|| kind_token(kind, locale)),
        written_key,
    )
}

/// Drop every filter token and keep the plain words.
pub fn strip_filters(text: &str) -> String {
    join(
        tokens(text)
            .into_iter()
            .filter(|t| split_token(t).1.is_none())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{GitFacts, Holder, Ward};

    const DAY: u64 = 86_400_000;

    fn facts(tone: Tone) -> Facts<'static> {
        Facts {
            tone,
            marked: false,
            now: 100 * DAY,
            name: "shown-name",
        }
    }

    fn wt() -> Husk {
        let mut h = Husk::bare(HuskKind::Worktree, "/home/me/dev/app-wt/fix-bug", 600 << 20);
        h.mtime_ms = Some(60 * DAY);
        h.agent = Some(AgentKind::Claude);
        h.git = Some(GitFacts {
            branch: Some("fix/Bug-12".into()),
            ..Default::default()
        });
        h
    }

    fn q(text: &str, h: &Husk, tone: Tone) -> bool {
        Query::parse(text).matches(h, &facts(tone))
    }

    #[test]
    fn tokens_keep_quotes_together() {
        assert_eq!(
            tokens(r#"  a  "b c" -path:"d e"  "#),
            vec!["a", r#""b c""#, r#"-path:"d e""#]
        );
        assert!(tokens("   ").is_empty());
    }

    #[test]
    fn plain_words_all_must_match() {
        let h = wt();
        assert!(q("", &h, Tone::Free));
        assert!(q("app fix", &h, Tone::Free));
        assert!(q("bug-12", &h, Tone::Free), "branch");
        assert!(q("claude", &h, Tone::Free), "agent");
        assert!(q("shown", &h, Tone::Free), "display name");
        assert!(!q("app zzz", &h, Tone::Free));
        assert!(!q("-app", &h, Tone::Free));
        assert!(q("-zzz", &h, Tone::Free));
        assert!(q(r#""fix-bug""#, &h, Tone::Free));
        assert!(q("-", &h, Tone::Free), "a lone minus is text");
    }

    #[test]
    fn keys_in_both_languages() {
        let h = wt();
        for yes in [
            "kind:worktree",
            "tipo:worktrees",
            "kind:deps,worktree",
            "agent:claude",
            "agente:CLAU",
            "size:>500mb",
            "peso:>=600",
            "tamanho:<1gb",
            "size:=600mb",
            "age:>30d",
            "idade:>1m",
            "age:<2mo",
            "age:>4w",
            "idade:>3s",
            "age:>24h",
            "age:>10min",
            "age:<1y",
            "idade:<1a",
            "branch:fix/",
            "path:app-wt",
            "caminho:DEV",
            "is:removable",
            "status:livre",
            "-kind:deps",
            "-is:blocked",
            "age:>0,5d",
        ] {
            assert!(q(yes, &h, Tone::Free), "{yes}");
        }
        for no in [
            "kind:deps",
            "agent:codex",
            "size:>1gb",
            "size:<=500mb",
            "age:<30d",
            "branch:feat/",
            "path:nowhere",
            "is:blocked",
            "status:marcado",
            "eco:node",
            "size:banana",
            "kind:nope",
            "-agent:claude",
        ] {
            assert!(!q(no, &h, Tone::Free), "{no}");
        }
        let mut undated = wt();
        undated.mtime_ms = None;
        assert!(!q("age:>1d", &undated, Tone::Free), "no mtime, no age");
        assert!(q("tipo:", &h, Tone::Free), "an empty value is dropped");
    }

    #[test]
    fn status_values() {
        let mut h = wt();
        h.ward(Ward::Dirty { files: 2 });
        h.ward(Ward::Stranded { commits: 1 });
        h.ward(Ward::Orphaned { origin: None });
        h.contains_secrets = true;
        h.ward(Ward::Occupied {
            holders: vec![Holder {
                pid: 1,
                name: "codex".into(),
                cwd: h.path.clone(),
                agent: Some(AgentKind::Codex),
            }],
        });
        for s in [
            "semcommit",
            "unpushed",
            "orfao",
            "segredos",
            "emuso",
            "bloqueado",
        ] {
            assert!(q(&format!("status:{s}"), &h, Tone::Untouchable), "{s}");
        }
        assert!(
            q("agent:codex", &h, Tone::Untouchable),
            "holder agents count"
        );
        assert!(q("is:caution", &h, Tone::Caution));
        assert!(q("is:protected", &h, Tone::Guarded));
        let mut f = facts(Tone::Free);
        f.marked = true;
        assert!(Query::parse("is:marked").matches(&h, &f));
        for s in Status::ALL {
            for l in [Locale::En, Locale::PtBr] {
                assert_eq!(Status::parse(s.token(l)), Some(s));
            }
        }
        let mut deps = Husk::bare(HuskKind::Ballast, "/p/node_modules", 1);
        deps.ecosystem = Some(huskmap_core::Ecosystem::Node);
        assert!(q("eco:node kind:dependências", &deps, Tone::Free));
        let mut s = Husk::bare(HuskKind::Afterimage, "/h/.grok/sessions/%2Fx", 1);
        s.project = Some("/home/me/Documentos/pixeiro".into());
        assert!(q("pixeiro", &s, Tone::Free), "the project is searchable");
        assert!(q("path:documentos", &s, Tone::Free));
    }

    #[test]
    fn kinds_round_trip() {
        for k in HuskKind::all() {
            for l in [Locale::En, Locale::PtBr] {
                assert_eq!(parse_kind(kind_token(k, l)), Some(k));
            }
        }
        assert_eq!(
            Query::parse("x tipo:sessões,logs").kinds(),
            Some(vec![HuskKind::Afterimage, HuskKind::Debris])
        );
        assert_eq!(Query::parse("-kind:logs").kinds(), None);
        assert!(Query::parse("  ").is_empty());
    }

    #[test]
    fn units_and_bounds() {
        assert_eq!(parse_size("2k"), Some((Cmp::Ge, 2048)));
        assert_eq!(parse_size("<1b"), Some((Cmp::Lt, 1)));
        assert_eq!(parse_size("1tb").map(|s| s.1), Some(1 << 40));
        assert_eq!(parse_size("1zz"), None);
        assert_eq!(parse_size("gb"), None);
        assert_eq!(parse_age("1").map(|a| a.1), Some(DAY));
        assert_eq!(parse_age("2x"), None);
    }

    #[test]
    fn chips_edit_the_text() {
        let t = toggle_value("fix", Key::Kind, "deps", "tipo");
        assert_eq!(t, "fix tipo:deps");
        let t = toggle_value(&t, Key::Kind, "worktree", "tipo");
        assert_eq!(t, "fix tipo:deps,worktree");
        assert!(has_value(&t, Key::Kind, "dependencias"));
        let t = toggle_value(&t, Key::Kind, "ballast", "kind");
        assert_eq!(t, "fix tipo:worktree", "aliases toggle the same value off");
        let t = toggle_value(&t, Key::Kind, "worktree", "tipo");
        assert_eq!(t, "fix");
        assert!(!has_value(&t, Key::Kind, "worktree"));
        let t = set_single("a size:>1gb -size:<1b", Key::Size, Some(">100mb"), "size");
        assert_eq!(t, "a -size:<1b size:>100mb", "negated tokens are yours");
        assert!(has_value(&t, Key::Size, ">100MB"));
        assert!(!has_value(&t, Key::Size, "banana"));
        assert_eq!(set_single(&t, Key::Size, None, "size"), "a -size:<1b");
        assert!(has_value("age:>7d", Key::Age, ">1w"));
        assert!(has_value("agent:Claude", Key::Agent, "claude"));
        assert_eq!(strip_filters("a kind:deps \"b c\" -is:free"), "a \"b c\"");
    }

    #[test]
    fn only_kind_toggles_back_to_everything() {
        let t = only_kind("x", HuskKind::Ballast, Locale::PtBr, "tipo");
        assert_eq!(t, "x tipo:deps");
        let t = only_kind(&t, HuskKind::Toolchain, Locale::En, "kind");
        assert_eq!(t, "x kind:packages");
        assert_eq!(only_kind(&t, HuskKind::Toolchain, Locale::En, "kind"), "x");
    }
}
