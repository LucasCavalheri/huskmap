// English is the source of truth. `pt-BR.ts` must satisfy `Dictionary`, so `astro check`
// fails on a missing key. Same rules as the app (AGENTS.md, "Copy: plain words").
// Minimal on purpose: one idea per section, one sentence per idea.
export const en = {
  locale: "en",
  htmlLang: "en",
  ogLocale: "en_US",
  label: "English",
  short: "EN",

  meta: {
    title: "huskmap · see what AI coding agents left on your disk",
    description:
      "huskmap finds the worktrees, node_modules, sessions and caches that Claude Code, Codex, Cursor and other agents leave on Linux, and moves what is safe to remove to the trash.",
    ogTitle: "huskmap · what your AI agents left behind",
    ogDescription:
      "Finds what coding agents leave on your disk, protects your unsaved work, frees the rest. Linux, open source.",
  },

  nav: {
    label: "Main navigation",
    home: "huskmap home",
    language: "Language",
    install: "Install",
    github: "GitHub",
    other: "Ver em português",
  },

  hero: {
    eyebrow: "For Linux · open source",
    titleA: "Your agents left",
    titleB: "things behind.",
    lead: "huskmap finds the worktrees, node_modules and caches AI coding agents leave on your disk, and clears what is safe to remove.",
    copy: "Copy",
    copied: "Copied",
    github: "View on GitHub",
    skip: "Skip to content",
  },

  compare: {
    drag: "Drag to compare the light and dark themes",
    dark: "Dark",
    light: "Light",
    alt: {
      dark: "huskmap in the dark theme",
      light: "huskmap in the light theme",
    },
    caption: "Light or dark, or follow your desktop.",
  },

  points: [
    { icon: "search", name: "Finds", body: "Worktrees, dependencies, sessions and caches from every agent." },
    { icon: "locked", name: "Protects", body: "Anything in use, uncommitted or unpushed stays put." },
    { icon: "trash", name: "Frees", body: "The rest goes to the trash, so you can always restore it." },
  ],

  agents: "Works with",

  install: {
    title: "One line. Any Linux.",
    note: ".deb, .rpm, pacman, .apk or a portable .tar.gz, checked against SHA256SUMS.",
    releases: "Or download a package",
  },

  download: {
    title: "Download",
    lead: "Pick your processor, then your Linux. The file comes straight from GitHub.",
    archLabel: "Processor",
    arch: { x86_64: "Intel / AMD", aarch64: "ARM64" },
    families: {
      deb: "Debian, Ubuntu, Mint",
      rpm: "Fedora, openSUSE, RHEL",
      pacman: "Arch, Manjaro",
      apk: "Alpine",
      tar: "Gentoo, Void, NixOS, any Linux",
    },
    get: "Download",
    unavailable: "Not built for this processor",
    soonShort: "Soon",
    soon: "The first release is on its way. The downloads open here as soon as it is out.",
    version: "Version {v}",
    sums: "Checksums (SHA256SUMS)",
    notes: "Release notes",
  },

  footer: {
    license: "MIT License",
    releases: "Releases",
    marks: "Logos belong to their owners.",
  },
};

export type Dictionary = typeof en;
