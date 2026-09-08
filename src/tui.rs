use std::io;

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

use crate::{key_stats, KeyStats, Stats};

/// Launch the interactive TUI: enter alternate screen + raw mode, render
/// key statistics over a scrollable per-month table, restore the terminal
/// on every exit path before returning.
pub fn run(stats: &[Stats]) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, stats);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, stats: &[Stats]) -> io::Result<()> {
    let keys = key_stats(stats);
    let mut state = TableState::default();
    state.select(Some(0));

    loop {
        let page = page_step(terminal.size()?.height);
        terminal.draw(|f| ui(f, stats, &keys, &mut state))?;
        if let Event::Key(key) = event::read()? {
            // Auto-repeat fires both Press and Release in some terminals; only
            // act on the key-down edge so holding a key doesn't double-step.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let last = stats.len().saturating_sub(1);
            let current = state.selected().unwrap_or(0);
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => {
                    state.select(Some(current.saturating_add(1).min(last)));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.select(Some(current.saturating_sub(1)));
                }
                KeyCode::PageDown => {
                    state.select(Some(current.saturating_add(page).min(last)));
                }
                KeyCode::PageUp => {
                    state.select(Some(current.saturating_sub(page)));
                }
                _ => {}
            }
        }
    }
}

/// Approximate visible rows in the table area: terminal height minus the
/// header block and table borders. Used for PageUp/PageDown stepping.
fn page_step(height: u16) -> usize {
    height.saturating_sub(9).max(1) as usize
}

fn ui(f: &mut Frame, stats: &[Stats], keys: &KeyStats, state: &mut TableState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(f.area());

    let insolvency = match keys.first_insolvent_month {
        Some(m) => format!("month {m}"),
        None => "never".to_string(),
    };
    let header_text = format!(
        "final month: cash {:.2}, assets {:.2}, cashflow {:.2}\n\
         min cash: {:.2} at month {}\n\
         insolvent: {}\n\
         \n\
         j/k or arrows scroll · PageUp/PageDown by screen · q/Esc quits",
        keys.final_cash,
        keys.final_assets_value,
        keys.final_monthly_cashflow,
        keys.min_cash,
        keys.min_cash_month,
        insolvency,
    );
    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Key statistics"),
    );
    f.render_widget(header, chunks[0]);

    let rows: Vec<Row> = stats
        .iter()
        .enumerate()
        .map(|(month, s)| {
            Row::new(vec![
                Cell::from(month.to_string()),
                Cell::from(format!("{:.2}", s.cash)),
                Cell::from(format!("{:.2}", s.assets_value)),
                Cell::from(format!("{:.2}", s.monthly_cashflow)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(16),
        ],
    )
    .header(Row::new(vec![
        Cell::from("month"),
        Cell::from("cash"),
        Cell::from("assets_value"),
        Cell::from("monthly_cashflow"),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Monthly"))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(table, chunks[1], state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Stats;
    use ratatui::backend::TestBackend;

    fn frame_to_string(f: &mut Terminal<TestBackend>) -> String {
        let buf = f.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn header_shows_final_and_min() {
        let stats = vec![
            Stats { cash: 100.0, assets_value: 50.0, monthly_cashflow: 25.0 },
            Stats { cash: 75.0, assets_value: 60.0, monthly_cashflow: -25.0 },
        ];
        let keys = key_stats(&stats);
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        state.select(Some(0));
        terminal
            .draw(|f| ui(f, &stats, &keys, &mut state))
            .unwrap();
        let rendered = frame_to_string(&mut terminal);
        assert!(rendered.contains("Key statistics"), "rendered: {rendered}");
        assert!(rendered.contains("final month"), "rendered: {rendered}");
        assert!(rendered.contains("min cash"), "rendered: {rendered}");
        assert!(rendered.contains("insolvent: never"), "rendered: {rendered}");
    }

    #[test]
    fn header_shows_insolvency_when_negative() {
        let stats = vec![
            Stats { cash: 100.0, assets_value: 0.0, monthly_cashflow: 0.0 },
            Stats { cash: -10.0, assets_value: 0.0, monthly_cashflow: -110.0 },
        ];
        let keys = key_stats(&stats);
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        terminal
            .draw(|f| ui(f, &stats, &keys, &mut state))
            .unwrap();
        let rendered = frame_to_string(&mut terminal);
        assert!(rendered.contains("insolvent: month 1"), "rendered: {rendered}");
    }

    #[test]
    fn table_renders_every_month_with_index() {
        let stats: Vec<Stats> = (0..13)
            .map(|i| Stats {
                cash: i as f64,
                assets_value: 0.0,
                monthly_cashflow: 1.0,
            })
            .collect();
        let keys = key_stats(&stats);
        let backend = TestBackend::new(120, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        state.select(Some(12));
        terminal
            .draw(|f| ui(f, &stats, &keys, &mut state))
            .unwrap();
        let rendered = frame_to_string(&mut terminal);
        // header row + every data row
        assert!(rendered.contains("month"), "header row missing: {rendered}");
        for i in 0..13 {
            let label = format!("{i}");
            assert!(rendered.contains(&label), "missing month {i}: {rendered}");
        }
    }
}
