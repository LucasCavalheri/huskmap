import type { Dictionary } from "./en";

// Português do Brasil, escrito do jeito que dev fala. Termos técnicos que a gente usa em
// inglês (worktree, branch, commit, push, cache, log) ficam em inglês.
export const ptBR: Dictionary = {
  locale: "pt-BR",
  htmlLang: "pt-BR",
  ogLocale: "pt_BR",
  label: "Português",
  short: "PT",

  meta: {
    title: "huskmap · veja o que os agentes de IA deixaram no seu disco",
    description:
      "O huskmap encontra os worktrees, node_modules, pastas target, sessões e caches que Claude Code, Codex, Cursor, Grok e outros agentes deixam no Linux, mostra o que dá para apagar com segurança e manda para a lixeira.",
    ogTitle: "huskmap · o que os seus agentes de IA deixaram para trás",
    ogDescription:
      "Um mapa do disco para quem trabalha com agentes de código. Acha worktrees e dependências esquecidas, protege o trabalho não salvo e manda o resto para a lixeira. Só Linux, open source.",
  },

  nav: {
    label: "Navegação principal",
    home: "Início do huskmap",
    finds: "O que ele acha",
    safety: "Segurança",
    how: "Como funciona",
    install: "Instalar",
    github: "GitHub",
    download: "Baixar",
    other: "View in English",
  },

  hero: {
    eyebrow: "Linux · open source · feito em Rust",
    titleA: "Seus agentes deixaram",
    titleB: "coisas para trás.",
    titleC: "Veja quanto elas pesam.",
    lead:
      "Claude Code, Codex, Cursor e companhia espalham worktrees do git, um node_modules ou target em cada um, sessões antigas e caches pelo seu disco. O huskmap acha tudo isso, mostra o que dá para apagar e nunca mexe no seu trabalho não salvo.",
    install: "Instalar",
    copy: "Copiar",
    copied: "Copiado",
    github: "Ver no GitHub",
    note: "Escanear só lê. Nada sai do lugar até você confirmar, e aí vai para a lixeira.",
    dialLabel: "Mapa de sonar animado com os itens que o huskmap encontrou",
    tones: ["Pode apagar", "Olhe antes", "Protegido", "Bloqueado"],
    skip: "Pular para o conteúdo",
  },

  proof: [
    { value: "6", label: "tipos de sobra que ele conhece" },
    { value: "0", label: "arquivos apagados sem o seu ok" },
    { value: "1", label: "chamada de rede: ver se tem versão nova" },
    { value: "5", label: "formatos de pacote, qualquer distro" },
  ],

  showcase: {
    eyebrow: "O app",
    title: "Um sonar sobre o seu disco.",
    lead:
      "Cada ponto é um item. A fatia é o tipo, a distância do centro é há quanto tempo ele não muda, o tamanho é quanto espaço ocupa e a cor é o que você pode fazer com ele.",
    tabs: {
      map: "Mapa",
      list: "Lista e filtros",
      drawer: "Detalhes",
      confirm: "Confirmação",
      guide: "Guia embutido",
    },
    captions: {
      map: "O mapa abre primeiro. O que está em uso pulsa em vermelho, e o feixe varre o disco enquanto o scan roda.",
      list: "A lista ordena por tamanho, idade ou nome, e filtra por tipo, situação, agente, tamanho e idade.",
      drawer: "Abra qualquer item para ver a branch, o último commit, o remoto e exatamente o que protege ele.",
      confirm: "Nada sai do lugar até você ler o resumo e confirmar. Tudo vai para a lixeira do sistema.",
      guide: "Na primeira vez, o app explica tudo: o mapa, os seis tipos, as proteções e cada atalho.",
    },
    alt: {
      map: "Tela de mapa do huskmap com os itens agrupados por tipo em volta de um sonar",
      list: "Tela de lista do huskmap filtrada por worktrees e dependências",
      drawer: "Painel de detalhes do huskmap para um worktree em uso pelo Claude Code",
      confirm: "Confirmação do huskmap antes de mandar itens para a lixeira",
      guide: "Guia Como funciona do huskmap explicando como ler o mapa",
    },
  },

  finds: {
    eyebrow: "O que ele acha",
    title: "Seis tipos de sobra.",
    lead: "Cada um é conferido por marcadores, não pelo tamanho. Uma pasta target só conta se tiver um Cargo.toml do lado.",
    items: [
      {
        kind: "worktree",
        name: "Worktrees",
        body: "Cópias extras de um repositório que os agentes criam para trabalhar em paralelo, em ~/.codex/worktrees, .claude/worktrees e nas pastas dos seus projetos.",
        tags: ["git worktree", "clones de agente"],
      },
      {
        kind: "ballast",
        name: "Dependências",
        body: "node_modules, target, .venv, .next e .turbo, muitas vezes repetidos em cada worktree. O projeto recria tudo depois.",
        tags: ["node_modules", "target", ".venv"],
      },
      {
        kind: "toolchain",
        name: "Pacotes",
        body: "Cache dos downloads de npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go e Playwright. São baixados de novo quando precisar.",
        tags: ["~/.npm", "~/.cache/uv", "~/.cargo"],
      },
      {
        kind: "afterimage",
        name: "Sessões",
        body: "Histórico de conversa que os agentes guardam por projeto. O huskmap liga cada uma ao seu projeto e avisa quando o projeto não existe mais.",
        tags: ["~/.claude/projects", "~/.grok"],
      },
      {
        kind: "cache",
        name: "Caches de IA",
        body: "Runtimes, histórico de arquivos e caches de colagem que os agentes guardam para eles mesmos.",
        tags: ["codex-runtimes", "file-history"],
      },
      {
        kind: "debris",
        name: "Logs",
        body: "Logs dos agentes, saída de tarefas em segundo plano e histórico de prompts.",
        tags: ["*.log", "history.jsonl"],
      },
    ],
    agentsTitle: "Sabe onde estes agentes guardam as coisas",
  },

  safety: {
    eyebrow: "Segurança primeiro",
    title: "Ele não come o seu trabalho.",
    lead:
      "Antes de marcar qualquer coisa, e de novo logo antes de mover, o huskmap confere quem está usando a pasta e o que o git sabe sobre ela.",
    rules: [
      { tone: "blocked", name: "Em uso", body: "Um agente, editor ou terminal está com a pasta aberta. O huskmap lê o /proc para saber. Nunca é apagada, nem forçando." },
      { tone: "blocked", name: "Checkout principal", body: "A pasta original de um repositório nunca é apagada." },
      { tone: "blocked", name: "Arquivos secretos", body: "Arquivos .env e chaves que só existem ali seguram a pasta onde está." },
      { tone: "guarded", name: "Alterações sem commit", body: "Fica protegido. Dá para marcar mesmo assim, e os arquivos vão juntos para a lixeira." },
      { tone: "guarded", name: "Commits só aqui", body: "Commits que não estão em nenhuma outra branch nem no remoto. A branch continua no repositório de qualquer jeito." },
      { tone: "free", name: "Todo o resto", body: "Vai para a lixeira do freedesktop, então dá para restaurar. Worktrees também saem da lista do git." },
    ],
    promise: "Por padrão, só escaneia e planeja. Só muda algo quando você confirma, depois de conferir tudo de novo.",
  },

  how: {
    eyebrow: "Como funciona",
    title: "Quatro passos. Um deles é só ler.",
    steps: [
      { name: "Escanear", body: "O huskmap lê as pastas dos agentes, dos seus projetos e os caches de pacotes. Só lê." },
      { name: "Olhar", body: "Abra o mapa ou a lista. Clique num item para ver tamanho, idade, branch e o que protege ele." },
      { name: "Marcar", body: "Marque os itens, aperte x, ou filtre a lista e use “Marcar N que podem sair”. O painel soma o que você vai liberar." },
      { name: "Mandar para a lixeira", body: "Leia o resumo e confirme. Tudo é conferido de novo antes, e depois vai para a lixeira." },
    ],
  },

  filters: {
    eyebrow: "Filtros",
    title: "Clique num botão ou digite.",
    lead: "Os botões em cima da lista escrevem na busca, então tudo o que você clica também dá para digitar. Os filtros valem no mapa também, em português ou inglês.",
    examples: [
      { q: "tipo:deps peso:>500mb idade:>30d", note: "dependências grandes e antigas" },
      { q: "status:semcommit,sempush", note: "worktrees com trabalho guardado" },
      { q: "agente:claude -status:bloqueado", note: "o que o Claude deixou e pode sair" },
      { q: "kind:sessions is:orphan", note: "o mesmo, em inglês" },
    ],
  },

  cli: {
    eyebrow: "Também no terminal",
    title: "Dá para automatizar quando quiser.",
    lead: "O mesmo núcleo roda uma CLI com saída JSON, um mapa no terminal e um arquivo de plano que você revisa antes de aplicar. Numa máquina sem tela, o huskmap abre a versão de terminal.",
    commands: [
      { cmd: "huskmap scan", note: "lista o que os agentes deixaram, em uso primeiro" },
      { cmd: "huskmap scan --json", note: "o relatório completo para scripts" },
      { cmd: "huskmap plan --preset safe", note: "uma simulação que você lê antes" },
      { cmd: "huskmap apply --plan plan.json", note: "confere de novo e manda para a lixeira" },
      { cmd: "huskmap map", note: "navega pelo último scan num TUI" },
    ],
  },

  install: {
    eyebrow: "Instalar",
    title: "Qualquer Linux. Uma linha.",
    lead: "O script vê o seu processador e o gerenciador de pacotes, confere o download com o SHA256SUMS e instala. Se o huskmap estiver mandando arquivos para a lixeira, ele espera, e as suas marcações sobrevivem à atualização.",
    or: "Ou baixe um pacote",
    formats: [
      { ext: ".deb", distros: "Debian, Ubuntu, Mint, Pop!_OS" },
      { ext: ".rpm", distros: "Fedora, RHEL, openSUSE" },
      { ext: ".pkg.tar.zst", distros: "Arch, Manjaro, EndeavourOS" },
      { ext: ".apk", distros: "Alpine" },
      { ext: ".tar.gz", distros: "Gentoo, Void, NixOS, qualquer um" },
    ],
    releases: "Todas as versões",
    user: "Sem root? Use --user para instalar em ~/.local.",
  },

  faq: {
    eyebrow: "Perguntas",
    title: "Antes de rodar.",
    items: [
      { q: "Ele apaga os meus arquivos?", a: "Não. Escanear só lê. Quando você confirma, os itens vão para a lixeira do sistema (o padrão de lixeira do freedesktop.org), e dá para restaurar pelo gerenciador de arquivos." },
      { q: "Ele manda algum dado para fora?", a: "Só uma requisição: uma vez por dia ele pergunta ao GitHub Releases se tem versão nova. Para desligar, use HUSKMAP_NO_UPDATE_CHECK=1. Sem telemetria, sem conta, sem geolocalização." },
      { q: "Quais agentes ele conhece?", a: "Claude Code, Codex, Cursor, Grok, Gemini CLI, OpenCode e Aider, além dos caches de pacotes do npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go e Playwright." },
      { q: "Por que só Linux?", a: "Porque ele lê o /proc para saber quem está trabalhando numa pasta, segue os diretórios XDG e usa a lixeira do freedesktop. Funciona em qualquer distro: glibc e musl, x86_64 e ARM64, X11 e Wayland." },
      { q: "É de graça?", a: "É. Licença MIT, código no GitHub." },
    ],
  },

  footer: {
    tagline: "A máquina tem uma vida secreta de agentes. Agora dá para ver.",
    license: "Licença MIT",
    releases: "Versões",
    issues: "Issues",
    marks: "Os logos de agentes e ferramentas pertencem aos donos e só identificam o que o huskmap encontra.",
  },
};
