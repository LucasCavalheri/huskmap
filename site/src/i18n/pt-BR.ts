import type { Dictionary } from "./en";

// Português do Brasil, do jeito que dev fala. Minimalista de propósito: uma ideia por seção.
export const ptBR: Dictionary = {
  locale: "pt-BR",
  htmlLang: "pt-BR",
  ogLocale: "pt_BR",
  label: "Português",
  short: "PT",

  meta: {
    title: "huskmap · veja o que os agentes de IA deixaram no seu disco",
    description:
      "O huskmap encontra os worktrees, node_modules, sessões e caches que Claude Code, Codex, Cursor e outros agentes deixam no Linux, e manda para a lixeira o que dá para apagar com segurança.",
    ogTitle: "huskmap · o que os seus agentes de IA deixaram para trás",
    ogDescription:
      "Acha o que os agentes de código deixam no seu disco, protege o que não foi salvo e libera o resto. Linux, open source.",
  },

  nav: {
    label: "Navegação principal",
    home: "Início do huskmap",
    language: "Idioma",
    install: "Instalar",
    github: "GitHub",
    other: "View in English",
  },

  hero: {
    eyebrow: "Para Linux · open source",
    titleA: "Seus agentes deixaram",
    titleB: "coisas para trás.",
    lead: "O huskmap acha os worktrees, node_modules e caches que os agentes de IA deixam no seu disco, e limpa o que dá para apagar com segurança.",
    copy: "Copiar",
    copied: "Copiado",
    github: "Ver no GitHub",
    skip: "Pular para o conteúdo",
  },

  compare: {
    drag: "Arraste para comparar o tema claro e o escuro",
    dark: "Escuro",
    light: "Claro",
    alt: {
      dark: "huskmap no tema escuro",
      light: "huskmap no tema claro",
    },
    caption: "Claro ou escuro, ou igual ao seu desktop.",
  },

  points: [
    { icon: "search", name: "Acha", body: "Worktrees, dependências, sessões e caches de todos os agentes." },
    { icon: "locked", name: "Protege", body: "O que está em uso, sem commit ou sem push fica onde está." },
    { icon: "trash", name: "Libera", body: "O resto vai para a lixeira, e dá para restaurar sempre." },
  ],

  agents: "Funciona com",

  install: {
    title: "Uma linha. Qualquer Linux.",
    note: ".deb, .rpm, pacman, .apk ou um .tar.gz portátil, conferidos com o SHA256SUMS.",
    releases: "Ou baixe um pacote",
  },

  download: {
    title: "Baixar",
    lead: "Escolha o processador, depois o seu Linux. O arquivo vem direto do GitHub.",
    archLabel: "Processador",
    arch: { x86_64: "Intel / AMD", aarch64: "ARM64" },
    families: {
      deb: "Debian, Ubuntu, Mint",
      rpm: "Fedora, openSUSE, RHEL",
      pacman: "Arch, Manjaro",
      apk: "Alpine",
      tar: "Gentoo, Void, NixOS, qualquer Linux",
    },
    get: "Baixar",
    unavailable: "Sem build para este processador",
    soonShort: "Em breve",
    soon: "A primeira versão está a caminho. Os downloads abrem aqui assim que ela sair.",
    version: "Versão {v}",
    sums: "Checksums (SHA256SUMS)",
    notes: "Novidades",
  },

  footer: {
    license: "Licença MIT",
    releases: "Versões",
    marks: "Os logos pertencem aos donos.",
  },
};
