//! Product copy. English first, PT-BR second. One module so a third locale can land later.
//! Never say "cleaner", "gc", or "agent-gc".

use std::sync::atomic::{AtomicU8, Ordering};

use crate::domain::{HuskKind, Risk, Ward, format_bytes};
use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Locale {
    En = 0,
    PtBr = 1,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-br",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, Error> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(Error::safety("unknown language"));
        }
        let lower = trimmed.to_ascii_lowercase();
        let base = lower
            .split('.')
            .next()
            .unwrap_or(lower.as_str())
            .replace('_', "-");
        if matches!(base.as_str(), "c" | "posix" | "english")
            || base == "en"
            || base.starts_with("en-")
        {
            return Ok(Self::En);
        }
        if base == "pt"
            || base.starts_with("pt-")
            || matches!(base.as_str(), "portuguese" | "portugues" | "português")
        {
            return Ok(Self::PtBr);
        }
        Err(Error::safety(format!("unknown language: {trimmed}")))
    }

    fn from_tag(tag: u8) -> Self {
        match tag {
            1 => Self::PtBr,
            _ => Self::En,
        }
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One entry in the "How it works" guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideMark {
    Plain,
    /// A numbered step.
    Step(u8),
    Kind(HuskKind),
    /// 0 removable, 1 check first, 2 protected, 3 blocked.
    Tone(u8),
    /// A key on the keyboard.
    Key,
    /// A filter you can type.
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuideItem {
    pub mark: GuideMark,
    pub term: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuideSection {
    pub title: &'static str,
    pub lead: &'static str,
    pub items: &'static [GuideItem],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guide {
    pub title: &'static str,
    pub sections: &'static [GuideSection],
}

const fn item(mark: GuideMark, term: &'static str, text: &'static str) -> GuideItem {
    GuideItem { mark, term, text }
}

use GuideMark::{Code, Key, Kind, Plain, Step, Tone};

/// All user-facing strings for one locale. `{x}` marks a substitution.
///
/// Labels use words people already know. The product's voice lives in taglines and empty
/// states, never in a button or a column name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deck {
    // voice
    pub about: &'static str,
    pub splash: &'static str,
    pub scan_hint: &'static str,
    pub empty_grove: &'static str,
    pub clean_grove: &'static str,
    pub scanning: &'static str,
    pub error_grove: &'static str,
    pub map_title: &'static str,
    pub map_subtitle: &'static str,
    // refusals
    pub no_report: &'static str,
    pub plan_required: &'static str,
    pub stale_plan: &'static str,
    pub wait_scan: &'static str,
    pub dirty_worktree: &'static str,
    pub unpushed_worktree: &'static str,
    pub worktree_locked: &'static str,
    pub primary_checkout: &'static str,
    pub outside_roots: &'static str,
    pub forbidden_path: &'static str,
    pub occupied_refuse: &'static str,
    pub already_gone: &'static str,
    pub nothing_to_apply: &'static str,
    pub prune_failed: &'static str,
    pub apply_busy: &'static str,
    pub applying_now: &'static str,
    pub update_unmanaged: &'static str,
    pub update_checksum: &'static str,
    pub update_no_pkexec: &'static str,
    pub update_declined: &'static str,
    pub update_available: &'static str,
    pub update_title: &'static str,
    pub update_now: &'static str,
    pub update_later: &'static str,
    pub update_skip: &'static str,
    pub update_stop: &'static str,
    pub updating: &'static str,
    pub update_done: &'static str,
    pub update_failed: &'static str,
    pub up_to_date: &'static str,
    pub update_during_apply: &'static str,
    pub help_update: &'static str,
    pub scan_thread_died: &'static str,
    pub gui_missing: &'static str,
    // notes
    pub note_debris: &'static str,
    pub note_history: &'static str,
    pub note_cache: &'static str,
    pub note_afterimage: &'static str,
    pub note_worktree_root: &'static str,
    pub note_git_worktree: &'static str,
    pub note_agent_clone: &'static str,
    pub note_empty_slot: &'static str,
    pub note_with_marker: &'static str,
    pub note_toolchain: &'static str,
    // wards
    pub ward_occupied: &'static str,
    pub ward_neighbor: &'static str,
    pub ward_dirty: &'static str,
    pub ward_stranded: &'static str,
    pub ward_locked: &'static str,
    pub ward_primary: &'static str,
    pub ward_secrets: &'static str,
    pub ward_outside: &'static str,
    pub ward_warm: &'static str,
    pub ward_no_git: &'static str,
    pub ward_orphaned: &'static str,
    // kinds (plural, singular)
    pub kind_worktree: (&'static str, &'static str),
    pub kind_ballast: (&'static str, &'static str),
    pub kind_toolchain: (&'static str, &'static str),
    pub kind_afterimage: (&'static str, &'static str),
    pub kind_cache: (&'static str, &'static str),
    pub kind_debris: (&'static str, &'static str),
    pub risk_safe: &'static str,
    pub risk_caution: &'static str,
    pub risk_dangerous: &'static str,
    pub risk_forbidden: &'static str,
    // what you may do with an item
    pub tone_free: &'static str,
    pub tone_caution: &'static str,
    pub tone_guarded: &'static str,
    pub tone_untouchable: &'static str,
    // counts
    pub reclaimable: &'static str,
    pub guarded: &'static str,
    pub marked: &'static str,
    pub husks: &'static str,
    pub groves: &'static str,
    pub seen_of: &'static str,
    pub alarms_title: &'static str,
    pub alarms_none: &'static str,
    pub alarms_count: &'static str,
    // actions
    pub sound_the_grove: &'static str,
    pub sound_again: &'static str,
    pub walking: &'static str,
    pub weighing: &'static str,
    pub view_map: &'static str,
    pub view_ledger: &'static str,
    pub mark: &'static str,
    pub unmark: &'static str,
    pub force_mark: &'static str,
    pub force_unmark: &'static str,
    pub force_hint: &'static str,
    pub forced_note: &'static str,
    pub open_folder: &'static str,
    pub copy_path: &'static str,
    pub copied: &'static str,
    pub open_failed: &'static str,
    pub detail_work: &'static str,
    pub work_safe: &'static str,
    pub work_stranded: &'static str,
    pub apply_open: &'static str,
    pub apply_title: &'static str,
    pub apply_title_one: &'static str,
    pub apply_confirm: &'static str,
    pub apply_guarded_note: &'static str,
    pub apply_withdraw: &'static str,
    pub apply_commit: &'static str,
    pub apply_summary: &'static str,
    pub apply_skip: &'static str,
    pub apply_err: &'static str,
    pub search_placeholder: &'static str,
    pub search_none: &'static str,
    // filters
    pub filter_kind: &'static str,
    pub filter_status: &'static str,
    pub filter_agent: &'static str,
    pub filter_size: &'static str,
    pub filter_age: &'static str,
    pub filter_any: &'static str,
    pub filter_clear: &'static str,
    pub filter_count: &'static str,
    pub filter_hint: &'static str,
    pub flag_occupied: &'static str,
    pub flag_dirty: &'static str,
    pub flag_stranded: &'static str,
    pub flag_secrets: &'static str,
    pub flag_orphaned: &'static str,
    pub flag_marked: &'static str,
    pub age_day: &'static str,
    pub age_week: &'static str,
    pub age_month: &'static str,
    pub age_quarter: &'static str,
    pub mark_visible: &'static str,
    pub unmark_visible: &'static str,
    pub sort_name: &'static str,
    pub col_age: &'static str,
    /// Query keys this locale writes when a chip is pressed: kind, status, agent, size, age.
    pub query_keys: [&'static str; 5],
    // detail
    pub detail_path: &'static str,
    pub detail_kind: &'static str,
    pub detail_risk: &'static str,
    pub detail_size: &'static str,
    pub detail_age: &'static str,
    pub detail_agent: &'static str,
    pub detail_branch: &'static str,
    pub detail_head: &'static str,
    pub detail_upstream: &'static str,
    pub detail_grove: &'static str,
    pub detail_project: &'static str,
    pub detail_wards: &'static str,
    pub detail_notes: &'static str,
    pub detail_duplicates: &'static str,
    pub detached: &'static str,
    pub no_upstream: &'static str,
    pub verdict_free: &'static str,
    pub verdict_guarded: &'static str,
    pub verdict_absolute: &'static str,
    // map rings
    pub ring_today: &'static str,
    pub ring_week: &'static str,
    pub ring_month: &'static str,
    pub ring_older: &'static str,
    pub ago: &'static str,
    // help
    pub guide_open: &'static str,
    pub guide_close: &'static str,
    pub guide: &'static Guide,
    pub keyhint: &'static str,
    pub help_scan: &'static str,
    pub help_doctor: &'static str,
    pub help_plan: &'static str,
    pub help_apply: &'static str,
    pub help_map: &'static str,
    pub help_gui: &'static str,
    pub help_lang: &'static str,
    pub doctor_home_missing: &'static str,
    pub doctor_no_roots: &'static str,
    pub doctor_no_proc: &'static str,
    pub doctor_git_ok: &'static str,
    pub doctor_root_open: &'static str,
    pub doctor_root_dark: &'static str,
    pub doctor_ok: &'static str,
    pub plan_header: &'static str,
    pub action_trash: &'static str,
    pub action_worktree_remove: &'static str,
    pub lang_en: &'static str,
    pub lang_pt: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
}

pub const BINARY: &str = "huskmap";

const GUIDE_EN: Guide = Guide {
    title: "How huskmap works",
    sections: &[
        GuideSection {
            title: "What it is for",
            lead: "AI coding agents (Claude Code, Codex, Cursor, Grok, Gemini, OpenCode, Aider) leave things on your disk: extra git worktrees, a node_modules or target folder in each of them, old sessions, caches and logs. huskmap finds all of it, shows how much space each item takes, and tells you what is safe to remove.",
            items: &[
                item(
                    Plain,
                    "It only reads",
                    "Scanning never changes anything. Nothing is removed until you mark items and confirm.",
                ),
                item(
                    Plain,
                    "It uses the trash",
                    "Removed items go to the system trash, so you can restore them from your file manager.",
                ),
                item(
                    Plain,
                    "It protects work",
                    "Folders in use, uncommitted changes, commits that exist nowhere else and secret files are protected.",
                ),
            ],
        },
        GuideSection {
            title: "Step by step",
            lead: "The whole flow takes four steps.",
            items: &[
                item(
                    Step(1),
                    "Scan",
                    "Press Scan (or s). huskmap reads agent folders, your project folders and package caches.",
                ),
                item(
                    Step(2),
                    "Look",
                    "Click an item on the map or in the list (or press enter) to see its size, age, branch and what protects it.",
                ),
                item(
                    Step(3),
                    "Mark",
                    "Tick the box, press x, or right-click a dot on the map. The left panel adds up what you marked.",
                ),
                item(
                    Step(4),
                    "Move to trash",
                    "Press Move to trash (or a), read the summary and confirm. huskmap checks everything again before it moves anything.",
                ),
            ],
        },
        GuideSection {
            title: "Reading the map",
            lead: "Each dot is one item. Where it sits, how big it is and its color all mean something.",
            items: &[
                item(
                    Plain,
                    "Slice",
                    "Which type the item is. The six slices match the six types in the left panel.",
                ),
                item(
                    Plain,
                    "Distance from the center",
                    "How long since it last changed. Near the center is recent, the outer edge is a year or more.",
                ),
                item(
                    Plain,
                    "Dot size",
                    "How much space it takes. Bigger dot, more space.",
                ),
                item(Tone(0), "Green", "Safe to remove."),
                item(Tone(1), "Amber", "Can be removed, but take a look first."),
                item(
                    Tone(2),
                    "Copper",
                    "Protected. You can still mark it anyway if you are sure.",
                ),
                item(
                    Tone(3),
                    "Red",
                    "Blocked. In use, the main checkout, or holds secrets. huskmap will not touch it.",
                ),
                item(
                    Plain,
                    "Pulsing ring",
                    "An agent or program is working in that folder right now.",
                ),
            ],
        },
        GuideSection {
            title: "The six types",
            lead: "Everything huskmap finds falls into one of these.",
            items: &[
                item(
                    Kind(HuskKind::Worktree),
                    "Worktrees",
                    "Extra copies of a repo that agents create to work in parallel (git worktree).",
                ),
                item(
                    Kind(HuskKind::Ballast),
                    "Dependencies",
                    "node_modules, target, .venv, .next and similar build or install folders. Your project recreates them with npm install, cargo build and so on.",
                ),
                item(
                    Kind(HuskKind::Toolchain),
                    "Package caches",
                    "Download caches of npm, pnpm, bun, yarn, pip, uv, cargo, go and Playwright. They are downloaded again when needed.",
                ),
                item(
                    Kind(HuskKind::Afterimage),
                    "Sessions",
                    "Conversation history that agents keep per project.",
                ),
                item(
                    Kind(HuskKind::Cache),
                    "Agent caches",
                    "Temporary files the agents keep for themselves.",
                ),
                item(
                    Kind(HuskKind::Debris),
                    "Logs",
                    "Agent logs and prompt history.",
                ),
            ],
        },
        GuideSection {
            title: "What protects an item",
            lead: "Shown in red or copper on the item. huskmap re-checks all of these right before moving anything.",
            items: &[
                item(
                    Tone(3),
                    "In use",
                    "An agent, editor or terminal has this folder open. Close it first.",
                ),
                item(
                    Tone(3),
                    "Main checkout",
                    "The repo's original folder. Never removed.",
                ),
                item(
                    Tone(3),
                    "Secret files",
                    ".env files and keys that exist only there.",
                ),
                item(
                    Tone(2),
                    "Uncommitted changes",
                    "Files changed but not committed. Mark anyway sends them to the trash too.",
                ),
                item(
                    Tone(2),
                    "Commits only here",
                    "Commits that are on no other branch or remote. The branch stays in the repo either way.",
                ),
                item(
                    Tone(1),
                    "Project gone",
                    "A session whose project folder no longer exists. Usually safe to remove.",
                ),
            ],
        },
        GuideSection {
            title: "Filters",
            lead: "Use the chips above the list, or type filters in the search box. Everything you type must match. Filters work on the map too.",
            items: &[
                item(
                    Code,
                    "kind:worktree",
                    "Only one type. Also: deps, packages, sessions, caches, logs. Separate several with commas.",
                ),
                item(Code, "agent:claude", "Only items from one agent."),
                item(
                    Code,
                    "size:>500mb",
                    "Bigger than 500 MB. Use <, >, <=, >= with b, kb, mb, gb.",
                ),
                item(
                    Code,
                    "age:>30d",
                    "Not changed for 30 days. Units: h, d, w, mo, y.",
                ),
                item(
                    Code,
                    "is:removable",
                    "By status: removable, caution, protected, blocked, inuse, uncommitted, unpushed, secrets, orphan, marked.",
                ),
                item(
                    Code,
                    "branch:fix/ path:api",
                    "Text inside the branch name or the path.",
                ),
                item(
                    Code,
                    "-node_modules",
                    "A minus sign excludes. Works with any filter: -agent:codex.",
                ),
                item(Code, "\"two words\"", "Quotes keep words together."),
            ],
        },
        GuideSection {
            title: "Keyboard",
            lead: "Everything has a key.",
            items: &[
                item(Key, "j / k", "Next / previous item"),
                item(Key, "enter", "Open details"),
                item(Key, "x", "Mark or unmark"),
                item(Key, "X", "Mark anyway (protected items)"),
                item(Key, "a", "Move marked items to the trash"),
                item(Key, "/", "Search and filter"),
                item(Key, "1 to 6", "Show only one type"),
                item(Key, "tab", "Switch between map and list"),
                item(Key, "s", "Scan again"),
                item(Key, "esc", "Close, or clear filters"),
                item(Key, "t", "Theme: system, light or dark"),
                item(Key, "?", "This guide"),
            ],
        },
    ],
};

const GUIDE_PT: Guide = Guide {
    title: "Como o huskmap funciona",
    sections: &[
        GuideSection {
            title: "Para que serve",
            lead: "Agentes de código com IA (Claude Code, Codex, Cursor, Grok, Gemini, OpenCode, Aider) deixam coisas no seu disco: worktrees do git a mais, uma pasta node_modules ou target em cada um deles, sessões antigas, caches e logs. O huskmap encontra tudo isso, mostra quanto espaço cada item ocupa e diz o que dá para apagar com segurança.",
            items: &[
                item(
                    Plain,
                    "Só lê",
                    "Escanear não muda nada. Nada é apagado até você marcar os itens e confirmar.",
                ),
                item(
                    Plain,
                    "Usa a lixeira",
                    "O que você apaga vai para a lixeira do sistema, e dá para restaurar pelo gerenciador de arquivos.",
                ),
                item(
                    Plain,
                    "Protege seu trabalho",
                    "Pastas em uso, alterações sem commit, commits que não existem em outro lugar e arquivos secretos ficam protegidos.",
                ),
            ],
        },
        GuideSection {
            title: "Passo a passo",
            lead: "O fluxo inteiro tem quatro passos.",
            items: &[
                item(
                    Step(1),
                    "Escanear",
                    "Clique em Escanear (ou aperte s). O huskmap lê as pastas dos agentes, dos seus projetos e os caches de pacotes.",
                ),
                item(
                    Step(2),
                    "Olhar",
                    "Clique num item do mapa ou da lista (ou aperte enter) para ver tamanho, idade, branch e o que o protege.",
                ),
                item(
                    Step(3),
                    "Marcar",
                    "Marque a caixinha, aperte x ou clique com o botão direito num ponto do mapa. O painel da esquerda soma o que você marcou.",
                ),
                item(
                    Step(4),
                    "Mandar para a lixeira",
                    "Clique em Mandar para a lixeira (ou aperte a), leia o resumo e confirme. O huskmap confere tudo de novo antes de mover qualquer coisa.",
                ),
            ],
        },
        GuideSection {
            title: "Como ler o mapa",
            lead: "Cada ponto é um item. A posição, o tamanho e a cor dizem alguma coisa.",
            items: &[
                item(
                    Plain,
                    "Fatia",
                    "O tipo do item. As seis fatias são os seis tipos do painel da esquerda.",
                ),
                item(
                    Plain,
                    "Distância do centro",
                    "Há quanto tempo ele não muda. Perto do centro é recente, a borda é um ano ou mais.",
                ),
                item(
                    Plain,
                    "Tamanho do ponto",
                    "Quanto espaço ocupa. Ponto maior, mais espaço.",
                ),
                item(Tone(0), "Verde", "Pode apagar sem medo."),
                item(Tone(1), "Âmbar", "Pode apagar, mas dê uma olhada antes."),
                item(
                    Tone(2),
                    "Cobre",
                    "Protegido. Se tiver certeza, dá para marcar mesmo assim.",
                ),
                item(
                    Tone(3),
                    "Vermelho",
                    "Bloqueado. Está em uso, é o checkout principal ou tem arquivos secretos. O huskmap não mexe.",
                ),
                item(
                    Plain,
                    "Anel pulsando",
                    "Um agente ou programa está trabalhando nessa pasta agora.",
                ),
            ],
        },
        GuideSection {
            title: "Os seis tipos",
            lead: "Tudo o que o huskmap encontra cai em um destes.",
            items: &[
                item(
                    Kind(HuskKind::Worktree),
                    "Worktrees",
                    "Cópias extras de um repositório que os agentes criam para trabalhar em paralelo (git worktree).",
                ),
                item(
                    Kind(HuskKind::Ballast),
                    "Dependências",
                    "node_modules, target, .venv, .next e pastas parecidas de build ou instalação. O projeto recria com npm install, cargo build e afins.",
                ),
                item(
                    Kind(HuskKind::Toolchain),
                    "Pacotes",
                    "Cache dos downloads do npm, pnpm, bun, yarn, pip, uv, cargo, go e Playwright. São baixados de novo quando precisar.",
                ),
                item(
                    Kind(HuskKind::Afterimage),
                    "Sessões",
                    "Histórico de conversas que os agentes guardam por projeto.",
                ),
                item(
                    Kind(HuskKind::Cache),
                    "Caches de IA",
                    "Arquivos temporários que os agentes guardam para eles mesmos.",
                ),
                item(
                    Kind(HuskKind::Debris),
                    "Logs",
                    "Logs dos agentes e histórico de prompts.",
                ),
            ],
        },
        GuideSection {
            title: "O que protege um item",
            lead: "Aparece em vermelho ou cobre no item. O huskmap confere tudo isso de novo logo antes de mover qualquer coisa.",
            items: &[
                item(
                    Tone(3),
                    "Em uso",
                    "Um agente, editor ou terminal está com essa pasta aberta. Feche antes.",
                ),
                item(
                    Tone(3),
                    "Checkout principal",
                    "A pasta original do repositório. Nunca é apagada.",
                ),
                item(
                    Tone(3),
                    "Arquivos secretos",
                    "Arquivos .env e chaves que só existem ali.",
                ),
                item(
                    Tone(2),
                    "Alterações sem commit",
                    "Arquivos alterados e não commitados. Marcar mesmo assim manda eles para a lixeira junto.",
                ),
                item(
                    Tone(2),
                    "Commits só aqui",
                    "Commits que não estão em nenhuma outra branch nem no remoto. A branch continua no repositório de qualquer jeito.",
                ),
                item(
                    Tone(1),
                    "Projeto apagado",
                    "Uma sessão cujo projeto não existe mais. Normalmente dá para apagar.",
                ),
            ],
        },
        GuideSection {
            title: "Filtros",
            lead: "Use os botões em cima da lista, ou digite filtros na busca. Tudo o que você digitar precisa bater. Os filtros valem no mapa também.",
            items: &[
                item(
                    Code,
                    "tipo:worktree",
                    "Só um tipo. Também: deps, pacotes, sessoes, caches, logs. Separe vários com vírgula.",
                ),
                item(Code, "agente:claude", "Só os itens de um agente."),
                item(
                    Code,
                    "peso:>500mb",
                    "Maior que 500 MB. Use <, >, <=, >= com b, kb, mb, gb.",
                ),
                item(
                    Code,
                    "idade:>30d",
                    "Sem mudar há 30 dias. Unidades: h (horas), d (dias), s (semanas), m (meses), a (anos).",
                ),
                item(
                    Code,
                    "status:livre",
                    "Pela situação: livre, atencao, protegido, bloqueado, emuso, semcommit, sempush, segredos, orfao, marcado.",
                ),
                item(
                    Code,
                    "branch:fix/ caminho:api",
                    "Texto dentro do nome da branch ou do caminho.",
                ),
                item(
                    Code,
                    "-node_modules",
                    "Um sinal de menos exclui. Vale para qualquer filtro: -agente:codex.",
                ),
                item(
                    Code,
                    "\"duas palavras\"",
                    "Aspas mantêm as palavras juntas.",
                ),
            ],
        },
        GuideSection {
            title: "Teclado",
            lead: "Tudo tem um atalho.",
            items: &[
                item(Key, "j / k", "Próximo / anterior"),
                item(Key, "enter", "Abrir detalhes"),
                item(Key, "x", "Marcar ou desmarcar"),
                item(Key, "X", "Marcar mesmo assim (itens protegidos)"),
                item(Key, "a", "Mandar os marcados para a lixeira"),
                item(Key, "/", "Buscar e filtrar"),
                item(Key, "1 a 6", "Mostrar só um tipo"),
                item(Key, "tab", "Trocar entre mapa e lista"),
                item(Key, "s", "Escanear de novo"),
                item(Key, "esc", "Fechar, ou limpar os filtros"),
                item(Key, "t", "Tema: sistema, claro ou escuro"),
                item(Key, "?", "Este guia"),
            ],
        },
    ],
};

pub const EN: Deck = Deck {
    about: "Find and free the disk space AI coding agents leave behind.",
    splash: "Your agents left things behind. Here is what they weigh.",
    scan_hint: "Scanning only reads. Nothing is removed until you confirm.",
    empty_grove: "Nothing scanned yet. huskmap reads agent folders, project folders and package caches to find what takes space. It only reads: nothing is removed until you mark items and confirm.",
    clean_grove: "Nothing to free. Your agents cleaned up after themselves, for once.",
    scanning: "Scanning",
    error_grove: "The scan failed.",
    map_title: "huskmap",
    map_subtitle: "what AI agents left on your disk",
    no_report: "No scan yet. Run huskmap scan first.",
    plan_required: "apply needs a plan file. Run huskmap plan first.",
    stale_plan: "Changed since the scan",
    wait_scan: "The scan is still running. Wait for it to finish.",
    dirty_worktree: "Has uncommitted changes. Needs --force.",
    unpushed_worktree: "Has commits that exist nowhere else. Needs --force.",
    worktree_locked: "Git has this worktree locked. Needs --force.",
    primary_checkout: "Main checkout of the repo. Never removed.",
    outside_roots: "Outside the scanned folders. Never removed.",
    forbidden_path: "Secret files are never touched.",
    occupied_refuse: "{who} is using this folder. Close it first.",
    already_gone: "already gone",
    nothing_to_apply: "Nothing marked can be removed.",
    prune_failed: "moved to the trash, but git still lists it; run git worktree prune",
    apply_busy: "Another huskmap is moving items to the trash (pid {pid}). Wait for it to finish.",
    applying_now: "Moving items to the trash",
    update_unmanaged: "This huskmap was installed with cargo or built from source; update it the same way.",
    update_checksum: "The download does not match SHA256SUMS. Refusing to install it.",
    update_no_pkexec: "Installing a package needs pkexec (polkit). Or run: curl -fsSL https://raw.githubusercontent.com/LucasCavalheri/huskmap/main/install.sh | bash",
    update_declined: "The password prompt was closed. Nothing changed.",
    update_available: "huskmap {v} is available",
    update_title: "A new version is available",
    update_now: "Update and restart",
    update_later: "Later",
    update_skip: "Skip {v}",
    update_stop: "Stop checking",
    updating: "Updating to {v}",
    update_done: "Updated to {v}.",
    update_failed: "Update failed: {err}",
    up_to_date: "huskmap {v} is the latest version.",
    update_during_apply: "Items are still moving to the trash. Update after that.",
    help_update: "Check for and install a newer huskmap",
    scan_thread_died: "The scan stopped unexpectedly.",
    gui_missing: "This build has no desktop app. Rebuild with --features gui.",
    note_debris: "agent log",
    note_history: "prompt history",
    note_cache: "agent cache",
    note_afterimage: "agent sessions",
    note_worktree_root: "agent worktree",
    note_git_worktree: "linked git worktree",
    note_agent_clone: "repo cloned by an agent",
    note_empty_slot: "empty worktree folder",
    note_with_marker: "{kind}, project file found next to it",
    note_toolchain: "{tool} cache, downloaded again on the next install",
    ward_occupied: "{who} is using this folder now",
    ward_neighbor: "{who} is running in this project",
    ward_dirty: "{n} uncommitted changes",
    ward_stranded: "{n} commits exist only here",
    ward_locked: "locked by git",
    ward_primary: "main checkout of the repo",
    ward_secrets: "has secret files (.env, keys)",
    ward_outside: "outside the scanned folders",
    ward_warm: "changed {n} min ago",
    ward_no_git: "not a git repo, so nothing proves these files are saved",
    ward_orphaned: "its project folder no longer exists",
    kind_worktree: ("Worktrees", "worktree"),
    kind_ballast: ("Dependencies", "dependency folder"),
    kind_toolchain: ("Package caches", "package cache"),
    kind_afterimage: ("Sessions", "agent sessions"),
    kind_cache: ("Agent caches", "agent cache"),
    kind_debris: ("Logs", "log"),
    risk_safe: "safe",
    risk_caution: "caution",
    risk_dangerous: "dangerous",
    risk_forbidden: "blocked",
    tone_free: "Safe to remove",
    tone_caution: "Check first",
    tone_guarded: "Protected",
    tone_untouchable: "Blocked",
    reclaimable: "can be freed",
    guarded: "protected",
    marked: "marked",
    husks: "items",
    groves: "repos",
    seen_of: "of {total} found",
    alarms_title: "In use or unsaved work",
    alarms_none: "No worktree is in use or holding unsaved work.",
    alarms_count: "{n} worktrees need attention",
    sound_the_grove: "Scan",
    sound_again: "Scan again",
    walking: "Reading {root}",
    weighing: "Measuring {n} items",
    view_map: "Map",
    view_ledger: "List",
    mark: "Mark",
    unmark: "Unmark",
    force_mark: "Mark anyway",
    force_unmark: "Unmark",
    force_hint: "Its files go to the trash; the branch stays in the repo.",
    forced_note: "Marked anyway: {n}. Uncommitted files go to the trash with them; branches stay in the repo.",
    open_folder: "Open folder",
    copy_path: "Copy path",
    copied: "Path copied.",
    open_failed: "No file manager opened (xdg-open failed).",
    detail_work: "commits",
    work_safe: "every commit is also on another branch",
    work_stranded: "{n} commits exist only here",
    apply_open: "Move to trash",
    apply_title: "Move {n} items to the trash?",
    apply_title_one: "Move this item to the trash?",
    apply_confirm: "{bytes} will go to the system trash, where you can restore it. Worktrees are also removed from git's list. Anything that changed since the scan is skipped.",
    apply_guarded_note: "{n} marked items are protected and will be skipped.",
    apply_withdraw: "Cancel",
    apply_commit: "Move to trash",
    apply_summary: "{bytes} freed · {skipped} skipped · {errors} errors",
    apply_skip: "skipped",
    apply_err: "error",
    search_placeholder: "Search or filter, e.g. size:>1gb",
    search_none: "No items match.",
    filter_kind: "type",
    filter_status: "status",
    filter_agent: "agent",
    filter_size: "size",
    filter_age: "unchanged for",
    filter_any: "any",
    filter_clear: "Clear filters",
    filter_count: "{n} of {total} items · {bytes}",
    filter_hint: "You can also type filters: size:>500mb age:>30d -node_modules. Press ? for all of them.",
    flag_occupied: "In use",
    flag_dirty: "Uncommitted",
    flag_stranded: "Unpushed",
    flag_secrets: "Secrets",
    flag_orphaned: "Project gone",
    flag_marked: "Marked",
    age_day: "1 day",
    age_week: "1 week",
    age_month: "1 month",
    age_quarter: "3 months",
    mark_visible: "Mark {n} removable",
    unmark_visible: "Unmark {n}",
    sort_name: "name",
    col_age: "age",
    query_keys: ["kind", "is", "agent", "size", "age"],
    detail_path: "path",
    detail_kind: "type",
    detail_risk: "risk",
    detail_size: "size",
    detail_age: "last changed",
    detail_agent: "agent",
    detail_branch: "branch",
    detail_head: "last commit",
    detail_upstream: "remote branch",
    detail_grove: "repo",
    detail_project: "project",
    detail_wards: "protections",
    detail_notes: "notes",
    detail_duplicates: "same folder in {n} other worktrees",
    detached: "no branch (detached)",
    no_upstream: "never pushed",
    verdict_free: "Safe to remove.",
    verdict_guarded: "Protected. You can still mark it anyway.",
    verdict_absolute: "Blocked. huskmap will not touch it.",
    ring_today: "1 day",
    ring_week: "1 week",
    ring_month: "1 month",
    ring_older: "1 year",
    ago: "ago",
    guide_open: "How it works",
    guide_close: "Got it",
    guide: &GUIDE_EN,
    keyhint: "j/k move  enter open  x mark  a trash  / filter  tab view  ? help",
    help_scan: "Scan agent and project folders and list what takes space",
    help_doctor: "Check that huskmap can scan this machine",
    help_plan: "Preview what a removal would do, from a scan report",
    help_apply: "Run a saved plan after checking everything again",
    help_map: "Browse the last scan in the terminal",
    help_gui: "Open the desktop app",
    help_lang: "Interface language: en or pt-br",
    doctor_home_missing: "home folder does not exist",
    doctor_no_roots: "none of the known folders exist yet",
    doctor_no_proc: "/proc is unreadable; huskmap cannot see which folders are in use",
    doctor_git_ok: "git works",
    doctor_root_open: "found",
    doctor_root_dark: "missing",
    doctor_ok: "Ready to scan.",
    plan_header: "plan",
    action_trash: "trash",
    action_worktree_remove: "worktree-remove",
    lang_en: "EN",
    lang_pt: "PT",
    theme_system: "System",
    theme_light: "Light",
    theme_dark: "Dark",
};

pub const PT_BR: Deck = Deck {
    about: "Encontra e libera o espaço que os agentes de IA deixam no seu disco.",
    splash: "Seus agentes deixaram coisas para trás. Veja quanto pesam.",
    scan_hint: "Escanear só lê. Nada é apagado até você confirmar.",
    empty_grove: "Nada escaneado ainda. O huskmap lê as pastas dos agentes, dos projetos e os caches de pacotes para achar o que ocupa espaço. Ele só lê: nada é apagado até você marcar os itens e confirmar.",
    clean_grove: "Nada para liberar. Seus agentes limparam a própria bagunça, pelo menos dessa vez.",
    scanning: "Escaneando",
    error_grove: "O scan falhou.",
    map_title: "huskmap",
    map_subtitle: "o que os agentes de IA deixaram no disco",
    no_report: "Nenhum scan ainda. Rode huskmap scan antes.",
    plan_required: "apply precisa de um arquivo de plano. Rode huskmap plan antes.",
    stale_plan: "Mudou desde o scan",
    wait_scan: "O scan ainda está rodando. Espere terminar.",
    dirty_worktree: "Tem alterações sem commit. Precisa de --force.",
    unpushed_worktree: "Tem commits que não existem em outro lugar. Precisa de --force.",
    worktree_locked: "O git travou este worktree. Precisa de --force.",
    primary_checkout: "Checkout principal do repositório. Nunca é apagado.",
    outside_roots: "Fora das pastas escaneadas. Nunca é apagado.",
    forbidden_path: "Arquivos secretos nunca são tocados.",
    occupied_refuse: "{who} está usando esta pasta. Feche antes.",
    already_gone: "já não existe",
    nothing_to_apply: "Nada do que foi marcado pode ser apagado.",
    prune_failed: "foi para a lixeira, mas o git ainda lista; rode git worktree prune",
    apply_busy: "Outro huskmap está mandando itens para a lixeira (pid {pid}). Espere terminar.",
    applying_now: "Mandando itens para a lixeira",
    update_unmanaged: "Este huskmap foi instalado pelo cargo ou compilado do código; atualize do mesmo jeito.",
    update_checksum: "O download não bate com o SHA256SUMS. A instalação foi recusada.",
    update_no_pkexec: "Instalar o pacote precisa do pkexec (polkit). Ou rode: curl -fsSL https://raw.githubusercontent.com/LucasCavalheri/huskmap/main/install.sh | bash",
    update_declined: "A janela de senha foi fechada. Nada mudou.",
    update_available: "huskmap {v} disponível",
    update_title: "Tem versão nova",
    update_now: "Atualizar e reiniciar",
    update_later: "Depois",
    update_skip: "Pular a {v}",
    update_stop: "Não verificar mais",
    updating: "Atualizando para a {v}",
    update_done: "Atualizado para a {v}.",
    update_failed: "A atualização falhou: {err}",
    up_to_date: "O huskmap {v} é a versão mais recente.",
    update_during_apply: "Ainda tem item indo para a lixeira. Atualize depois.",
    help_update: "Procura e instala uma versão nova do huskmap",
    scan_thread_died: "O scan parou do nada.",
    gui_missing: "Este build não tem o app desktop. Recompile com --features gui.",
    note_debris: "log de agente",
    note_history: "histórico de prompts",
    note_cache: "cache de agente",
    note_afterimage: "sessões de agente",
    note_worktree_root: "worktree de agente",
    note_git_worktree: "worktree git vinculado",
    note_agent_clone: "repositório clonado por agente",
    note_empty_slot: "pasta de worktree vazia",
    note_with_marker: "{kind}, com o arquivo do projeto ao lado",
    note_toolchain: "cache do {tool}, baixado de novo na próxima instalação",
    ward_occupied: "{who} está usando esta pasta agora",
    ward_neighbor: "{who} está rodando neste projeto",
    ward_dirty: "{n} alterações sem commit",
    ward_stranded: "{n} commits só existem aqui",
    ward_locked: "travado pelo git",
    ward_primary: "checkout principal do repositório",
    ward_secrets: "tem arquivos secretos (.env, chaves)",
    ward_outside: "fora das pastas escaneadas",
    ward_warm: "alterado há {n} min",
    ward_no_git: "não é um repositório git, nada garante que esses arquivos estão salvos",
    ward_orphaned: "a pasta do projeto não existe mais",
    kind_worktree: ("Worktrees", "worktree"),
    kind_ballast: ("Dependências", "pasta de dependências"),
    kind_toolchain: ("Pacotes", "cache de pacotes"),
    kind_afterimage: ("Sessões", "sessões de agente"),
    kind_cache: ("Caches de IA", "cache de agente"),
    kind_debris: ("Logs", "log"),
    risk_safe: "seguro",
    risk_caution: "atenção",
    risk_dangerous: "perigoso",
    risk_forbidden: "bloqueado",
    tone_free: "Pode apagar",
    tone_caution: "Olhe antes",
    tone_guarded: "Protegido",
    tone_untouchable: "Bloqueado",
    reclaimable: "dá para liberar",
    guarded: "protegido",
    marked: "marcados",
    husks: "itens",
    groves: "repositórios",
    seen_of: "de {total} encontrados",
    alarms_title: "Em uso ou com trabalho não salvo",
    alarms_none: "Nenhum worktree em uso ou com trabalho não salvo.",
    alarms_count: "{n} worktrees precisam de atenção",
    sound_the_grove: "Escanear",
    sound_again: "Escanear de novo",
    walking: "Lendo {root}",
    weighing: "Medindo {n} itens",
    view_map: "Mapa",
    view_ledger: "Lista",
    mark: "Marcar",
    unmark: "Desmarcar",
    force_mark: "Marcar mesmo assim",
    force_unmark: "Desmarcar",
    force_hint: "Os arquivos vão para a lixeira; a branch continua no repositório.",
    forced_note: "Marcados mesmo assim: {n}. Arquivos sem commit vão junto para a lixeira; as branches continuam no repositório.",
    open_folder: "Abrir pasta",
    copy_path: "Copiar caminho",
    copied: "Caminho copiado.",
    open_failed: "Nenhum gerenciador de arquivos abriu (o xdg-open falhou).",
    detail_work: "commits",
    work_safe: "todo commit também está em outra branch",
    work_stranded: "{n} commits só existem aqui",
    apply_open: "Mandar para a lixeira",
    apply_title: "Mandar {n} itens para a lixeira?",
    apply_title_one: "Mandar este item para a lixeira?",
    apply_confirm: "{bytes} vão para a lixeira do sistema, de onde dá para restaurar. Os worktrees também saem da lista do git. O que mudou desde o scan é pulado.",
    apply_guarded_note: "{n} itens marcados estão protegidos e serão pulados.",
    apply_withdraw: "Cancelar",
    apply_commit: "Mandar para a lixeira",
    apply_summary: "{bytes} liberados · {skipped} pulados · {errors} erros",
    apply_skip: "pulado",
    apply_err: "erro",
    search_placeholder: "Buscar ou filtrar, ex: peso:>1gb",
    search_none: "Nenhum item encontrado.",
    filter_kind: "tipo",
    filter_status: "situação",
    filter_agent: "agente",
    filter_size: "tamanho",
    filter_age: "sem mudar há",
    filter_any: "qualquer",
    filter_clear: "Limpar filtros",
    filter_count: "{n} de {total} itens · {bytes}",
    filter_hint: "Dá para digitar filtros também: peso:>500mb idade:>30d -node_modules. Aperte ? para ver todos.",
    flag_occupied: "Em uso",
    flag_dirty: "Sem commit",
    flag_stranded: "Sem push",
    flag_secrets: "Segredos",
    flag_orphaned: "Projeto apagado",
    flag_marked: "Marcados",
    age_day: "1 dia",
    age_week: "1 semana",
    age_month: "1 mês",
    age_quarter: "3 meses",
    mark_visible: "Marcar {n} que podem sair",
    unmark_visible: "Desmarcar {n}",
    sort_name: "nome",
    col_age: "idade",
    query_keys: ["tipo", "status", "agente", "peso", "idade"],
    detail_path: "caminho",
    detail_kind: "tipo",
    detail_risk: "risco",
    detail_size: "tamanho",
    detail_age: "última alteração",
    detail_agent: "agente",
    detail_branch: "branch",
    detail_head: "último commit",
    detail_upstream: "branch remota",
    detail_grove: "repositório",
    detail_project: "projeto",
    detail_wards: "proteções",
    detail_notes: "notas",
    detail_duplicates: "mesma pasta em {n} outros worktrees",
    detached: "sem branch (detached)",
    no_upstream: "nunca teve push",
    verdict_free: "Pode apagar.",
    verdict_guarded: "Protegido. Dá para marcar mesmo assim.",
    verdict_absolute: "Bloqueado. O huskmap não mexe nisso.",
    ring_today: "1 dia",
    ring_week: "1 semana",
    ring_month: "1 mês",
    ring_older: "1 ano",
    ago: "atrás",
    guide_open: "Como funciona",
    guide_close: "Entendi",
    guide: &GUIDE_PT,
    keyhint: "j/k mover  enter abrir  x marcar  a lixeira  / filtrar  tab visão  ? ajuda",
    help_scan: "Escaneia as pastas dos agentes e dos projetos e lista o que ocupa espaço",
    help_doctor: "Confere se o huskmap consegue escanear esta máquina",
    help_plan: "Mostra o que uma remoção faria, a partir de um relatório",
    help_apply: "Executa um plano salvo depois de conferir tudo de novo",
    help_map: "Navega pelo último scan no terminal",
    help_gui: "Abre o app desktop",
    help_lang: "Idioma da interface: en ou pt-br",
    doctor_home_missing: "a pasta home não existe",
    doctor_no_roots: "nenhuma das pastas conhecidas existe ainda",
    doctor_no_proc: "/proc ilegível; o huskmap não consegue ver quais pastas estão em uso",
    doctor_git_ok: "git funciona",
    doctor_root_open: "encontrada",
    doctor_root_dark: "não existe",
    doctor_ok: "Pronto para escanear.",
    plan_header: "plano",
    action_trash: "lixeira",
    action_worktree_remove: "remover-worktree",
    lang_en: "EN",
    lang_pt: "PT",
    theme_system: "Sistema",
    theme_light: "Claro",
    theme_dark: "Escuro",
};

pub const ABOUT: &str = EN.about;
pub const SPLASH: &str = EN.splash;

static LOCALE: AtomicU8 = AtomicU8::new(0);

thread_local! {
    static OVERRIDE: std::cell::Cell<Option<Locale>> = const { std::cell::Cell::new(None) };
}

pub fn current_locale() -> Locale {
    OVERRIDE
        .with(|o| o.get())
        .unwrap_or_else(|| Locale::from_tag(LOCALE.load(Ordering::Relaxed)))
}

/// Run `f` with a locale for this thread only. Tests use it so they never race the global.
pub fn with_locale<R>(locale: Locale, f: impl FnOnce() -> R) -> R {
    let prev = OVERRIDE.with(|o| o.replace(Some(locale)));
    let out = f();
    OVERRIDE.with(|o| o.set(prev));
    out
}

pub fn set_locale(locale: Locale) {
    LOCALE.store(locale as u8, Ordering::Relaxed);
}

pub fn get() -> &'static Deck {
    deck_for(current_locale())
}

pub fn deck_for(locale: Locale) -> &'static Deck {
    match locale {
        Locale::En => &EN,
        Locale::PtBr => &PT_BR,
    }
}

/// Brazilian IANA zones. `Brazil/*` legacy links included.
const BRAZIL_ZONES: &[&str] = &[
    "America/Sao_Paulo",
    "America/Araguaina",
    "America/Bahia",
    "America/Belem",
    "America/Boa_Vista",
    "America/Campo_Grande",
    "America/Cuiaba",
    "America/Eirunepe",
    "America/Fortaleza",
    "America/Maceio",
    "America/Manaus",
    "America/Noronha",
    "America/Porto_Velho",
    "America/Recife",
    "America/Rio_Branco",
    "America/Santarem",
];

/// Is this IANA zone in Brazil? Accepts `:America/Sao_Paulo` and zoneinfo paths.
pub fn zone_is_brazil(zone: &str) -> bool {
    let zone = zone.trim().trim_start_matches(':');
    let zone = zone.rsplit_once("zoneinfo/").map_or(zone, |(_, z)| z);
    let zone = zone
        .strip_prefix("posix/")
        .or_else(|| zone.strip_prefix("right/"))
        .unwrap_or(zone);
    zone.starts_with("Brazil/") || BRAZIL_ZONES.contains(&zone)
}

/// Is this POSIX locale tagged with the BR region? `pt_BR.UTF-8`, `en_BR`.
pub fn locale_is_brazil(raw: &str) -> bool {
    let base = raw.trim().split(['.', '@']).next().unwrap_or("");
    base.split(['_', '-'])
        .nth(1)
        .is_some_and(|region| region.eq_ignore_ascii_case("br"))
}

/// Everything the machine says about where its person is. No network, ever.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocaleSignals {
    /// `--lang` or `HUSKMAP_LANG`.
    pub explicit: Option<String>,
    /// What the person picked last time in the app.
    pub saved: Option<String>,
    /// `LC_ALL`, `LC_MESSAGES`, `LANG` values, in that order.
    pub posix: Vec<String>,
    /// IANA zone from `TZ`, `/etc/localtime` or `/etc/timezone`.
    pub zone: Option<String>,
}

/// Explicit wins, then the saved pick, then geography: Brazil reads pt-BR, everyone else English.
pub fn decide(signals: &LocaleSignals) -> Locale {
    for raw in [&signals.explicit, &signals.saved].into_iter().flatten() {
        if let Ok(locale) = Locale::parse(raw) {
            return locale;
        }
    }
    let in_brazil = signals.zone.as_deref().is_some_and(zone_is_brazil)
        || signals.posix.iter().any(|l| locale_is_brazil(l));
    if in_brazil { Locale::PtBr } else { Locale::En }
}

/// Read the zone the way glibc and musl do: `TZ`, then `/etc/localtime`, then `/etc/timezone`.
pub fn system_zone(tz: Option<String>, etc: &std::path::Path) -> Option<String> {
    if let Some(tz) = tz.filter(|t| !t.trim().is_empty()) {
        return Some(tz);
    }
    if let Ok(target) = std::fs::read_link(etc.join("localtime")) {
        return Some(target.to_string_lossy().into_owned());
    }
    std::fs::read_to_string(etc.join("timezone"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The language picked in the app, from `settings.json`.
pub fn load_saved(config_dir: &std::path::Path) -> Option<String> {
    crate::settings::Settings::load(config_dir)
        .lang
        .filter(|s| !s.trim().is_empty())
}

/// Remember a pick from the app's language switch.
pub fn save_choice(config_dir: &std::path::Path, locale: Locale) -> Result<(), Error> {
    crate::settings::Settings::update(config_dir, |s| s.lang = Some(locale.as_str().into()))
        .map(|_| ())
}

pub fn signals_from_env(
    args_lang: Option<String>,
    config_dir: &std::path::Path,
    etc: &std::path::Path,
    var: impl Fn(&str) -> Option<String>,
) -> LocaleSignals {
    LocaleSignals {
        explicit: args_lang.or_else(|| var("HUSKMAP_LANG")),
        saved: load_saved(config_dir),
        posix: ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|k| var(k))
            .filter(|v| !v.is_empty())
            .collect(),
        zone: system_zone(var("TZ"), etc),
    }
}

pub fn peek_lang_from_args<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        let arg = arg.as_ref();
        if arg == "--lang" {
            return iter.next().map(|s| s.as_ref().to_string());
        }
        if let Some(value) = arg.strip_prefix("--lang=") {
            return Some(value.to_string());
        }
    }
    None
}

/// Decide the locale once at process start from args, env, the saved pick and the zone.
pub fn init() {
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let config = crate::roots::Dirs::current(home).config;
    let signals = signals_from_env(
        peek_lang_from_args(std::env::args().skip(1)),
        &config,
        std::path::Path::new("/etc"),
        |k| std::env::var(k).ok(),
    );
    set_locale(decide(&signals));
}

pub fn kind_label(kind: HuskKind, plural: bool) -> &'static str {
    let d = get();
    let pair = match kind {
        HuskKind::Worktree => d.kind_worktree,
        HuskKind::Ballast => d.kind_ballast,
        HuskKind::Toolchain => d.kind_toolchain,
        HuskKind::Afterimage => d.kind_afterimage,
        HuskKind::Cache => d.kind_cache,
        HuskKind::Debris => d.kind_debris,
    };
    if plural { pair.0 } else { pair.1 }
}

pub fn risk_label(risk: Risk) -> &'static str {
    let d = get();
    match risk {
        Risk::Safe => d.risk_safe,
        Risk::Caution => d.risk_caution,
        Risk::Dangerous => d.risk_dangerous,
        Risk::Forbidden => d.risk_forbidden,
    }
}

fn who(holders: &[crate::domain::Holder]) -> String {
    match holders {
        [] => "?".into(),
        [one] => one.label(),
        [one, rest @ ..] => format!("{} +{}", one.label(), rest.len()),
    }
}

/// One line per ward, in the current locale.
pub fn ward_text(ward: &Ward) -> String {
    let d = get();
    match ward {
        Ward::Occupied { holders } => d.ward_occupied.replace("{who}", &who(holders)),
        Ward::NeighborBusy { holders } => d.ward_neighbor.replace("{who}", &who(holders)),
        Ward::Dirty { files } => d.ward_dirty.replace("{n}", &files.to_string()),
        Ward::Stranded { commits } => d.ward_stranded.replace("{n}", &commits.to_string()),
        Ward::Locked { reason: Some(r) } => format!("{}: {r}", d.ward_locked),
        Ward::Locked { reason: None } => d.ward_locked.into(),
        Ward::Primary => d.ward_primary.into(),
        Ward::Secrets => d.ward_secrets.into(),
        Ward::OutsideRoots => d.ward_outside.into(),
        Ward::Warm { minutes } => d.ward_warm.replace("{n}", &minutes.to_string()),
        Ward::NoGit => d.ward_no_git.into(),
        Ward::Orphaned { .. } => d.ward_orphaned.into(),
    }
}

/// Notes are written into the report in the language of the scan. Show the fixed ones in the
/// current language; templated or unknown notes pass through unchanged.
pub fn relocalize_note(note: &str) -> String {
    let pairs = |d: &Deck| {
        [
            d.note_debris,
            d.note_history,
            d.note_cache,
            d.note_afterimage,
            d.note_worktree_root,
            d.note_git_worktree,
            d.note_agent_clone,
            d.note_empty_slot,
        ]
    };
    let now = pairs(get());
    for deck in [&EN, &PT_BR] {
        if let Some(i) = pairs(deck).iter().position(|n| *n == note) {
            return now[i].to_string();
        }
    }
    note.to_string()
}

pub fn format_apply_summary(bytes: u64, skipped: usize, errors: usize) -> String {
    get()
        .apply_summary
        .replace("{bytes}", &format_bytes(bytes))
        .replace("{skipped}", &skipped.to_string())
        .replace("{errors}", &errors.to_string())
}

const BANNED: &[&str] = &["agent-gc", "cleaner", " gc ", "garbage collect", "oops"];

pub fn asserts_voice(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    !BANNED.iter().any(|b| lower.contains(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, Holder};
    use std::path::PathBuf;

    fn deck_strings(d: &Deck) -> String {
        format!("{d:?}")
    }

    #[test]
    fn voice_holds_in_both_locales() {
        for d in [&EN, &PT_BR] {
            let all = deck_strings(d);
            assert!(asserts_voice(&all), "{all}");
            assert!(!all.contains('—'), "no em dash in copy");
        }
        assert!(!asserts_voice("an agent-gc clone"));
        assert!(!asserts_voice("Oops!"));
    }

    /// Internal domain words (husk, grove, ballast) are for code. People read plain words.
    #[test]
    fn copy_uses_plain_words() {
        let jargon = [
            "husk ",
            "husks",
            "grove",
            "ballast",
            "afterimage",
            "ledger",
            "sound the",
            "sound again",
            "casca",
            "bosque",
            "lastro",
            "rastro",
            "sondar",
            "sondado",
            "livro",
            "detrito",
        ];
        for d in [&EN, &PT_BR] {
            // Only the quoted values: Debug also prints the field names.
            let all: String = deck_strings(d)
                .split('"')
                .skip(1)
                .step_by(2)
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
                .replace("lucascavalheri", "")
                .replace("huskmap", "");
            for word in jargon {
                assert!(!all.contains(word), "jargon {word:?} in copy");
            }
        }
    }

    #[test]
    fn notes_follow_the_language() {
        with_locale(Locale::PtBr, || {
            assert_eq!(
                relocalize_note(EN.note_git_worktree),
                PT_BR.note_git_worktree
            );
            assert_eq!(relocalize_note("git: boom"), "git: boom");
        });
        with_locale(Locale::En, || {
            assert_eq!(relocalize_note(PT_BR.note_debris), EN.note_debris);
        });
    }

    #[test]
    fn guides_cover_the_same_ground() {
        assert_eq!(EN.guide.sections.len(), PT_BR.guide.sections.len());
        for (en, pt) in EN.guide.sections.iter().zip(PT_BR.guide.sections) {
            assert_eq!(en.items.len(), pt.items.len(), "{}", en.title);
            for (a, b) in en.items.iter().zip(pt.items) {
                assert_eq!(a.mark, b.mark, "{} / {}", a.term, b.term);
                assert!(!a.text.is_empty() && !b.text.is_empty());
            }
        }
        for kind in HuskKind::all() {
            for d in [&EN, &PT_BR] {
                assert!(
                    d.guide
                        .sections
                        .iter()
                        .flat_map(|s| s.items)
                        .any(|i| i.mark == GuideMark::Kind(kind)),
                    "{kind:?} explained"
                );
            }
        }
    }

    #[test]
    fn decks_differ_where_they_should() {
        assert_ne!(EN.splash, PT_BR.splash);
        assert_eq!(EN.map_title, PT_BR.map_title);
        assert!(PT_BR.ward_stranded.contains("{n}"));
        assert!(EN.occupied_refuse.contains("{who}"));
    }

    #[test]
    fn parse_locales() {
        for raw in ["en", "EN_us.UTF-8", "C", "posix", "english", "en-GB"] {
            assert_eq!(Locale::parse(raw).unwrap(), Locale::En, "{raw}");
        }
        for raw in ["pt", "pt_BR.UTF-8", "pt-br", "português", "portuguese"] {
            assert_eq!(Locale::parse(raw).unwrap(), Locale::PtBr, "{raw}");
        }
        assert!(Locale::parse("").is_err());
        assert!(Locale::parse("klingon").is_err());
        assert_eq!(Locale::En.to_string(), "en");
        assert_eq!(Locale::PtBr.as_str(), "pt-br");
        assert_eq!(Locale::from_tag(9), Locale::En);
    }

    #[test]
    fn brazil_by_zone_or_region_everyone_else_english() {
        let sig = |zone: Option<&str>, posix: &[&str]| LocaleSignals {
            zone: zone.map(str::to_string),
            posix: posix.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(decide(&sig(None, &[])), Locale::En);
        assert_eq!(
            decide(&sig(Some("America/Sao_Paulo"), &["en_US.UTF-8"])),
            Locale::PtBr
        );
        assert_eq!(
            decide(&sig(Some("/usr/share/zoneinfo/America/Manaus"), &[])),
            Locale::PtBr
        );
        assert_eq!(decide(&sig(Some(":Brazil/East"), &[])), Locale::PtBr);
        assert_eq!(
            decide(&sig(Some("Europe/Lisbon"), &["pt_PT.UTF-8"])),
            Locale::En,
            "Portugal is not Brazil"
        );
        assert_eq!(
            decide(&sig(Some("America/New_York"), &["en_BR"])),
            Locale::PtBr,
            "region BR"
        );
        assert_eq!(
            decide(&sig(Some("America/Buenos_Aires"), &["C"])),
            Locale::En
        );
        assert!(zone_is_brazil("posix/America/Recife"));
        assert!(!zone_is_brazil("America/Santiago"));
        assert!(locale_is_brazil("pt_BR.UTF-8@euro"));
        assert!(locale_is_brazil("pt-br"));
        assert!(!locale_is_brazil("pt"));
    }

    #[test]
    fn explicit_then_saved_then_geography() {
        let mut s = LocaleSignals {
            explicit: Some("en".into()),
            saved: Some("pt-br".into()),
            zone: Some("America/Sao_Paulo".into()),
            posix: vec![],
        };
        assert_eq!(decide(&s), Locale::En);
        s.explicit = Some("klingon".into());
        assert_eq!(
            decide(&s),
            Locale::PtBr,
            "bad explicit falls through to saved"
        );
        s.explicit = None;
        s.saved = Some("en".into());
        assert_eq!(decide(&s), Locale::En, "a saved pick beats the zone");
    }

    #[test]
    fn zone_sources_and_saved_file() {
        let tmp = tempfile::tempdir().unwrap();
        let etc = tmp.path().join("etc");
        std::fs::create_dir_all(&etc).unwrap();
        assert_eq!(system_zone(None, &etc), None);
        std::fs::write(etc.join("timezone"), "America/Bahia\n").unwrap();
        assert_eq!(system_zone(None, &etc).as_deref(), Some("America/Bahia"));
        std::os::unix::fs::symlink("/usr/share/zoneinfo/Europe/Berlin", etc.join("localtime"))
            .unwrap();
        assert_eq!(
            system_zone(Some(" ".into()), &etc).as_deref(),
            Some("/usr/share/zoneinfo/Europe/Berlin")
        );
        assert_eq!(
            system_zone(Some("UTC".into()), &etc).as_deref(),
            Some("UTC")
        );

        let config = tmp.path().join("config");
        assert_eq!(load_saved(&config), None);
        save_choice(&config, Locale::PtBr).unwrap();
        assert_eq!(load_saved(&config).as_deref(), Some("pt-br"));
        let blocked = tmp.path().join("file");
        std::fs::write(&blocked, "x").unwrap();
        assert!(save_choice(&blocked, Locale::En).is_err());

        let env = |k: &str| match k {
            "LANG" => Some("en_US.UTF-8".to_string()),
            "LC_ALL" => Some(String::new()),
            "TZ" => Some("America/Fortaleza".to_string()),
            _ => None,
        };
        let s = signals_from_env(None, &config, &etc, env);
        assert_eq!(s.posix, vec!["en_US.UTF-8"]);
        assert_eq!(s.saved.as_deref(), Some("pt-br"));
        assert_eq!(s.zone.as_deref(), Some("America/Fortaleza"));
        let s = signals_from_env(Some("en".into()), &config, &etc, env);
        assert_eq!(decide(&s), Locale::En);
    }

    #[test]
    fn lang_from_args() {
        assert_eq!(peek_lang_from_args(["--lang", "pt"]), Some("pt".into()));
        assert_eq!(
            peek_lang_from_args(["scan", "--lang=en"]),
            Some("en".into())
        );
        assert_eq!(peek_lang_from_args(["scan"]), None);
        assert_eq!(peek_lang_from_args(["--lang"]), None);
    }

    #[test]
    fn labels_and_wards_follow_locale() {
        with_locale(Locale::PtBr, || {
            assert_eq!(current_locale(), Locale::PtBr);
            assert_eq!(kind_label(HuskKind::Afterimage, true), "Sessões");
            assert_eq!(risk_label(Risk::Caution), "atenção");
            assert_eq!(
                ward_text(&Ward::Stranded { commits: 3 }),
                "3 commits só existem aqui"
            );
        });
        with_locale(Locale::En, labels_and_wards_en);
    }

    #[test]
    fn global_locale_set_and_init() {
        std::thread::spawn(|| {
            set_locale(Locale::En);
            init();
            let _ = current_locale();
            set_locale(Locale::En);
        })
        .join()
        .unwrap();
    }

    fn labels_and_wards_en() {
        let holder = |pid| Holder {
            pid,
            name: "zsh".into(),
            cwd: PathBuf::from("/w"),
            agent: Some(AgentKind::Claude),
        };
        for kind in HuskKind::all() {
            assert!(!kind_label(kind, true).is_empty());
            assert!(!kind_label(kind, false).is_empty());
        }
        for risk in [Risk::Safe, Risk::Caution, Risk::Dangerous, Risk::Forbidden] {
            assert!(!risk_label(risk).is_empty());
        }
        let wards = [
            Ward::Occupied {
                holders: vec![holder(1), holder(2)],
            },
            Ward::NeighborBusy {
                holders: vec![holder(1)],
            },
            Ward::Occupied { holders: vec![] },
            Ward::Dirty { files: 2 },
            Ward::Locked {
                reason: Some("agent".into()),
            },
            Ward::Locked { reason: None },
            Ward::Primary,
            Ward::Secrets,
            Ward::OutsideRoots,
            Ward::Warm { minutes: 4 },
            Ward::NoGit,
            Ward::Orphaned { origin: None },
        ];
        let texts: Vec<String> = wards.iter().map(ward_text).collect();
        assert_eq!(texts[0], "claude · pid 1 +1 is using this folder now");
        assert_eq!(texts[1], "claude · pid 1 is running in this project");
        assert!(texts[2].contains('?'));
        assert_eq!(texts[4], "locked by git: agent");
        assert!(texts.iter().all(|t| !t.contains('{')));
        assert_eq!(
            format_apply_summary(2048, 1, 0),
            "2.0 KB freed · 1 skipped · 0 errors"
        );
        assert_eq!(deck_for(Locale::PtBr).lang_pt, "PT");
    }
}
