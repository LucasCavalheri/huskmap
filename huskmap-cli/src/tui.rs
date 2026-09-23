use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use huskmap_core::{Risk, ScanReport, copy};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::paint::Tone;
use crate::view::{HuskRow, ScanView};

fn c(t: Tone) -> Color {
    let (r, g, b) = t.rgb();
    Color::Rgb(r, g, b)
}

const CARBON: Color = Color::Rgb(10, 11, 10);

/// Keyboard state for the ledger. Pure so it is testable without a terminal.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub selected: usize,
    pub len: usize,
}

impl Cursor {
    pub fn key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.len.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = self.len.saturating_sub(1),
            _ => {}
        }
        true
    }
}

fn row_tone(row: &HuskRow) -> Tone {
    if row.alarm || matches!(row.risk, Risk::Dangerous | Risk::Forbidden) {
        Tone::Oxblood
    } else if row.reclaimable && row.risk == Risk::Safe {
        Tone::Verdigris
    } else {
        Tone::Amber
    }
}

/// Alarms first, then everything else by weight.
pub fn ledger(view: &ScanView) -> Vec<HuskRow> {
    let mut rows: Vec<HuskRow> = view.alarms.clone();
    rows.extend(view.rows.iter().filter(|r| !r.alarm).cloned());
    rows
}

pub fn detail_lines(row: &HuskRow) -> Vec<Line<'static>> {
    let deck = copy::get();
    let kv = |k: &str, v: String| {
        Line::from(vec![
            Span::styled(format!("{k:<10}"), Style::default().fg(c(Tone::Ash))),
            Span::styled(v, Style::default().fg(c(Tone::Bone))),
        ])
    };
    let mut lines = vec![
        Line::from(Span::styled(
            row.path.clone(),
            Style::default()
                .fg(c(Tone::Bone))
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        kv(deck.detail_kind, row.kind_label.clone()),
        kv(deck.detail_size, row.size.clone()),
        kv(deck.detail_age, row.age.clone()),
    ];
    if let Some(agent) = &row.agent {
        lines.push(kv(deck.detail_agent, agent.clone()));
    }
    if let Some(branch) = &row.branch {
        lines.push(kv(deck.detail_branch, branch.clone()));
    }
    lines.push(Line::raw(""));
    let verdict = if row.reclaimable {
        (deck.verdict_free, Tone::Verdigris)
    } else {
        (deck.verdict_guarded, Tone::Amber)
    };
    lines.push(Line::from(Span::styled(
        verdict.0,
        Style::default().fg(c(verdict.1)),
    )));
    for w in &row.wards {
        lines.push(Line::from(Span::styled(
            format!("↳ {w}"),
            Style::default().fg(c(row_tone(row))),
        )));
    }
    lines
}

pub fn run_map(report: &ScanReport) -> anyhow::Result<()> {
    let view = ScanView::from_report(report);
    let mut terminal = ratatui::init();
    let result = loop_ui(&mut terminal, &view);
    ratatui::restore();
    result
}

fn draw(frame: &mut ratatui::Frame, view: &ScanView, rows: &[HuskRow], cursor: &Cursor) {
    let deck = copy::get();
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(CARBON)), area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(area);

    let alarms = if view.alarms.is_empty() {
        Span::styled(deck.alarms_none, Style::default().fg(c(Tone::Ash)))
    } else {
        Span::styled(
            deck.alarms_count
                .replace("{n}", &view.alarms.len().to_string()),
            Style::default()
                .fg(c(Tone::Oxblood))
                .add_modifier(Modifier::BOLD),
        )
    };
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!("{}  ", view.title),
                Style::default()
                    .fg(c(Tone::Bone))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(view.subtitle.clone(), Style::default().fg(c(Tone::Copper))),
        ]),
        Line::from(vec![
            Span::styled(
                view.reclaimable_label.clone(),
                Style::default().fg(c(Tone::Amber)),
            ),
            Span::styled(
                format!("  {}   ", view.seen_label),
                Style::default().fg(c(Tone::Ash)),
            ),
            alarms,
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(c(Tone::Copper))),
    );
    frame.render_widget(header, chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(chunks[1]);
    draw_list(frame, body[0], view, rows, cursor);
    let detail = rows
        .get(cursor.selected)
        .map(detail_lines)
        .unwrap_or_else(|| vec![Line::raw(deck.empty_grove)]);
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(c(Tone::Copper)))
                .title(Span::styled(
                    format!(" {} ", deck.detail_wards),
                    Style::default().fg(c(Tone::Copper)),
                )),
        ),
        body[1],
    );
    frame.render_widget(
        Paragraph::new(format!("{}  q quit", deck.keyhint))
            .style(Style::default().fg(c(Tone::Ash))),
        chunks[2],
    );
}

fn draw_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    view: &ScanView,
    rows: &[HuskRow],
    cursor: &Cursor,
) {
    let deck = copy::get();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| {
            let tone = row_tone(row);
            ListItem::new(Line::from(vec![
                Span::styled(
                    if row.reclaimable { "● " } else { "■ " },
                    Style::default().fg(c(tone)),
                ),
                Span::styled(
                    format!("{:>9} ", row.size),
                    Style::default().fg(c(Tone::Bone)),
                ),
                Span::styled(
                    format!("{:>5}  ", row.age),
                    Style::default().fg(c(Tone::Ash)),
                ),
                Span::styled(
                    format!("{:<11} ", row.kind_label),
                    Style::default().fg(c(Tone::Copper)),
                ),
                Span::styled(row.path.clone(), Style::default().fg(c(Tone::Bone))),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(34, 30, 24))
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().title(Span::styled(
            format!(
                " {} {} · {} {} ",
                view.husk_count, deck.husks, view.grove_count, deck.groves
            ),
            Style::default().fg(c(Tone::Copper)),
        )));
    let mut state = ListState::default().with_selected(Some(cursor.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn loop_ui(terminal: &mut DefaultTerminal, view: &ScanView) -> anyhow::Result<()> {
    let rows = ledger(view);
    let mut cursor = Cursor {
        selected: 0,
        len: rows.len(),
    };
    loop {
        terminal.draw(|frame| draw(frame, view, &rows, &cursor))?;
        if event::poll(std::time::Duration::from_millis(250)).context("poll")?
            && let Event::Key(key) = event::read().context("read")?
            && key.kind == KeyEventKind::Press
            && !cursor.key(key.code)
        {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{Husk, HuskKind, Ward};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    fn report() -> ScanReport {
        let mut r = ScanReport::empty(PathBuf::from("/h"), 1);
        let big = Husk::bare(HuskKind::Cache, "/h/.codex/cache", 9000);
        let mut wt = Husk::bare(HuskKind::Worktree, "/h/dev/wt", 10);
        wt.ward(Ward::Dirty { files: 3 });
        r.husks = vec![big, wt];
        r
    }

    #[test]
    fn cursor_keys() {
        let mut cur = Cursor {
            selected: 0,
            len: 3,
        };
        assert!(cur.key(KeyCode::Char('j')));
        assert!(cur.key(KeyCode::Down));
        assert!(cur.key(KeyCode::Down));
        assert_eq!(cur.selected, 2);
        cur.key(KeyCode::Char('k'));
        assert_eq!(cur.selected, 1);
        cur.key(KeyCode::Char('g'));
        assert_eq!(cur.selected, 0);
        cur.key(KeyCode::End);
        assert_eq!(cur.selected, 2);
        cur.key(KeyCode::Up);
        cur.key(KeyCode::Home);
        cur.key(KeyCode::Char('z'));
        assert!(!cur.key(KeyCode::Char('q')));
        assert!(!cur.key(KeyCode::Esc));
        let mut empty = Cursor::default();
        empty.key(KeyCode::Down);
        assert_eq!(empty.selected, 0);
    }

    #[test]
    fn ledger_leads_with_alarms_and_renders() {
        copy::with_locale(huskmap_core::Locale::En, || {
            let view = ScanView::from_report(&report());
            let rows = ledger(&view);
            assert_eq!(rows[0].kind, HuskKind::Worktree);
            assert_eq!(rows.len(), 2);
            let lines = detail_lines(&rows[0]);
            let text: String = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("3 uncommitted changes"));
            assert!(text.contains("Protected"));
            let free = detail_lines(&rows[1]);
            assert!(
                free.iter()
                    .any(|l| l.to_string().contains("Safe to remove"))
            );
            let mut term = Terminal::new(TestBackend::new(120, 20)).unwrap();
            let cursor = Cursor {
                selected: 0,
                len: rows.len(),
            };
            term.draw(|f| draw(f, &view, &rows, &cursor)).unwrap();
            let buf = format!("{:?}", term.backend().buffer());
            assert!(buf.contains("huskmap"));
            let empty_view = ScanView::from_report(&ScanReport::empty(PathBuf::from("/h"), 1));
            term.draw(|f| draw(f, &empty_view, &[], &Cursor::default()))
                .unwrap();
        });
        assert_eq!(
            row_tone(&ledger(&ScanView::from_report(&report()))[1]),
            Tone::Verdigris
        );
    }
}
