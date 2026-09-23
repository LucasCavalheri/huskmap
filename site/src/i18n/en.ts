// English is the source of truth. `pt-BR.ts` must satisfy `Dictionary`, so `astro check`
// fails on a missing key. Same rules as the app (AGENTS.md, "Copy: plain words").
export const en = {
  locale: "en",
  htmlLang: "en",
  ogLocale: "en_US",
  label: "English",
  short: "EN",

  meta: {
    title: "huskmap · see what AI coding agents left on your disk",
    description:
      "huskmap finds the worktrees, node_modules, target folders, sessions and caches that Claude Code, Codex, Cursor, Grok and other agents leave on Linux, shows what is safe to remove and sends it to the trash.",
    ogTitle: "huskmap · what your AI agents left behind",
    ogDescription:
      "A disk map for people who work with coding agents. Finds leftover worktrees and dependencies, protects unsaved work, and moves the rest to the trash. Linux only, open source.",
  },

  nav: {
    label: "Main navigation",
    home: "huskmap home",
    finds: "What it finds",
    safety: "Safety",
    how: "How it works",
    install: "Install",
    github: "GitHub",
    download: "Download",
    other: "Ver em português",
  },

  hero: {
    eyebrow: "Linux · open source · written in Rust",
    titleA: "Your agents left",
    titleB: "things behind.",
    titleC: "Here is what they weigh.",
    lead:
      "Claude Code, Codex, Cursor and friends leave git worktrees, a node_modules or target in each one, old sessions and caches all over your disk. huskmap finds all of it, shows what is safe to remove and never touches your unsaved work.",
    install: "Install",
    copy: "Copy",
    copied: "Copied",
    github: "View on GitHub",
    note: "Scanning only reads. Nothing moves until you confirm, and then it goes to the trash.",
    dialLabel: "Animated sonar map of the items huskmap found",
    tones: ["Safe to remove", "Check first", "Protected", "Blocked"],
    skip: "Skip to content",
  },

  proof: [
    { value: "6", label: "types of leftovers it knows" },
    { value: "0", label: "files deleted without your OK" },
    { value: "1", label: "network call: the update check" },
    { value: "5", label: "package formats, every distro" },
  ],

  showcase: {
    eyebrow: "The app",
    title: "A sonar over your disk.",
    lead:
      "Every dot is one item. The slice is its type, the distance from the center is how long since it changed, the size is how much space it takes and the color is what you are allowed to do with it.",
    tabs: {
      map: "Map",
      list: "List and filters",
      drawer: "Details",
      confirm: "Confirm",
      guide: "Built-in guide",
    },
    captions: {
      map: "The map opens first. Items in use pulse in red; the beam sweeps the disk while a scan runs.",
      list: "The list sorts by size, age or name, and filters by type, status, agent, size and age.",
      drawer: "Open any item to see its branch, last commit, remote and exactly what protects it.",
      confirm: "Nothing moves until you read the summary and confirm. It all goes to the system trash.",
      guide: "The first launch explains everything: the map, the six types, the protections and every key.",
    },
    alt: {
      map: "huskmap map view with items grouped by type around a sonar dial",
      list: "huskmap list view filtered to worktrees and dependencies",
      drawer: "huskmap detail drawer for a worktree in use by Claude Code",
      confirm: "huskmap confirmation before moving items to the trash",
      guide: "huskmap How it works guide explaining how to read the map",
    },
  },

  finds: {
    eyebrow: "What it finds",
    title: "Six kinds of leftovers.",
    lead: "Each one is checked with markers, not guessed from size. A target folder counts only next to a Cargo.toml.",
    items: [
      {
        kind: "worktree",
        name: "Worktrees",
        body: "Extra copies of a repo that agents create to work in parallel, under ~/.codex/worktrees, .claude/worktrees and your project folders.",
        tags: ["git worktree", "agent clones"],
      },
      {
        kind: "ballast",
        name: "Dependencies",
        body: "node_modules, target, .venv, .next and .turbo, often duplicated once per worktree. Your project rebuilds them.",
        tags: ["node_modules", "target", ".venv"],
      },
      {
        kind: "toolchain",
        name: "Package caches",
        body: "Download caches for npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go and Playwright. They download again when needed.",
        tags: ["~/.npm", "~/.cache/uv", "~/.cargo"],
      },
      {
        kind: "afterimage",
        name: "Sessions",
        body: "Conversation history agents keep per project. huskmap matches each one to its project and flags the ones whose project is gone.",
        tags: ["~/.claude/projects", "~/.grok"],
      },
      {
        kind: "cache",
        name: "Agent caches",
        body: "Runtimes, file history and paste caches the agents keep for themselves.",
        tags: ["codex-runtimes", "file-history"],
      },
      {
        kind: "debris",
        name: "Logs",
        body: "Agent logs, background task output and prompt history.",
        tags: ["*.log", "history.jsonl"],
      },
    ],
    agentsTitle: "Knows where these agents keep things",
  },

  safety: {
    eyebrow: "Safety first",
    title: "It will not eat your work.",
    lead:
      "Before anything is marked, and again right before anything moves, huskmap checks who is using the folder and what git knows about it.",
    rules: [
      { tone: "blocked", name: "In use", body: "An agent, editor or terminal has the folder open. huskmap reads /proc to know. Never removed, not even by force." },
      { tone: "blocked", name: "Main checkout", body: "The original folder of a repo is never removed." },
      { tone: "blocked", name: "Secret files", body: ".env files and keys that exist only there keep the folder where it is." },
      { tone: "guarded", name: "Uncommitted changes", body: "Protected. You can still mark it anyway, and the files go to the trash with it." },
      { tone: "guarded", name: "Commits only here", body: "Commits that are on no other branch or remote. The branch stays in the repo either way." },
      { tone: "free", name: "Everything else", body: "Moves to the freedesktop trash, so you can restore it. Worktrees are also removed from git's list." },
    ],
    promise: "Scan and plan by default. Changes only on an explicit confirm, after every fact is checked again.",
  },

  how: {
    eyebrow: "How it works",
    title: "Four steps. One of them is reading.",
    steps: [
      { name: "Scan", body: "huskmap reads agent folders, your project folders and package caches. It only reads." },
      { name: "Look", body: "Open the map or the list. Click an item to see its size, age, branch and what protects it." },
      { name: "Mark", body: "Tick items, press x, or filter the list and press “Mark N removable”. The panel adds up what you would free." },
      { name: "Move to trash", body: "Read the summary and confirm. Everything is checked again first, then it goes to the trash." },
    ],
  },

  filters: {
    eyebrow: "Filters",
    title: "Click a chip or type a query.",
    lead: "The chips above the list write into the search box, so anything you click you could also type. Filters work on the map too, in English or Portuguese.",
    examples: [
      { q: "kind:deps size:>500mb age:>30d", note: "big, old dependency folders" },
      { q: "is:uncommitted,unpushed", note: "worktrees holding work" },
      { q: "agent:claude -is:blocked", note: "Claude's leftovers you may remove" },
      { q: "tipo:sessoes status:orfao", note: "the same, in Portuguese" },
    ],
  },

  cli: {
    eyebrow: "Also in the terminal",
    title: "Scriptable when you want it.",
    lead: "The same core runs a CLI with JSON output, a terminal map and a plan file you can review before applying. On a machine without a display, huskmap opens the terminal version.",
    commands: [
      { cmd: "huskmap scan", note: "list what agents left, in use first" },
      { cmd: "huskmap scan --json", note: "the full report for scripts" },
      { cmd: "huskmap plan --preset safe", note: "a dry run you can read" },
      { cmd: "huskmap apply --plan plan.json", note: "re-checks, then moves to trash" },
      { cmd: "huskmap map", note: "browse the last scan in a TUI" },
    ],
  },

  install: {
    eyebrow: "Install",
    title: "Every Linux. One line.",
    lead: "The script detects your processor and package manager, checks the download against SHA256SUMS and installs it. If huskmap is moving files to the trash, it waits; your marks survive the update.",
    or: "Or download a package",
    formats: [
      { ext: ".deb", distros: "Debian, Ubuntu, Mint, Pop!_OS" },
      { ext: ".rpm", distros: "Fedora, RHEL, openSUSE" },
      { ext: ".pkg.tar.zst", distros: "Arch, Manjaro, EndeavourOS" },
      { ext: ".apk", distros: "Alpine" },
      { ext: ".tar.gz", distros: "Gentoo, Void, NixOS, anything" },
    ],
    releases: "All releases",
    user: "No root? Add --user to install into ~/.local.",
  },

  faq: {
    eyebrow: "Questions",
    title: "Before you run it.",
    items: [
      { q: "Does it delete my files?", a: "No. Scanning only reads. When you confirm, items go to the system trash (the freedesktop.org trash spec), so you can restore them from your file manager." },
      { q: "Does it send anything anywhere?", a: "Only one request: a daily check against GitHub Releases for a newer version. Turn it off with HUSKMAP_NO_UPDATE_CHECK=1. No telemetry, no accounts, no geolocation." },
      { q: "Which agents does it know?", a: "Claude Code, Codex, Cursor, Grok, Gemini CLI, OpenCode and Aider, plus the package caches of npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go and Playwright." },
      { q: "Why Linux only?", a: "Because it reads /proc to see who is working in a folder, follows the XDG directories and uses the freedesktop trash. It supports every distro: glibc and musl, x86_64 and ARM64, X11 and Wayland." },
      { q: "Is it free?", a: "Yes. MIT license, source on GitHub." },
    ],
  },

  footer: {
    tagline: "The machine has a secret afterlife of agents. Now you can see it.",
    license: "MIT License",
    releases: "Releases",
    issues: "Issues",
    marks: "Agent and tool logos belong to their owners and only identify what huskmap finds.",
  },
};

export type Dictionary = typeof en;
