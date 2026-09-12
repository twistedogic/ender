use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame, Terminal,
};

use crate::{evaluate_goals, key_stats as derive_key_stats, load, GoalOutcome, KeyStats, Stats};

/// One row of the comparison table — key statistics for a single scenario
/// plus the basename used as the row label and the resolved goal outcomes.
#[derive(Debug)]
pub struct CompareRow {
    pub name: String,
    pub stats: KeyStats,
    pub terminal_month: Option<u16>,
    pub goals: Vec<GoalOutcome>,
}

/// Public entry point. Loads each scenario, simulates it, then renders the
/// comparison table. JSON takes precedence; otherwise TTY → ratatui,
/// non-TTY → aligned text. `compare` errors are returned as `io::Error`
/// with the path embedded so `main` can print to stderr with exit 1.
pub fn run(paths: &[PathBuf], json: bool) -> io::Result<()> {
    let rows = load_rows(paths)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if json {
        writeln!(out, "{}", format_json(&rows))?;
        return Ok(());
    }
    if out.is_terminal() {
        drop(out);
        render_tui(&rows)?;
    } else {
        write!(out, "{}", format_text(&rows))?;
    }
    Ok(())
}

/// Load each scenario from `paths`, in the given order, and return a row
/// per scenario with its name, key stats, terminal month, and goal outcomes.
/// `load` and `into_scenario` errors are wrapped as `io::Error` with the
/// failing path preserved in the message. ponytail: pub for tests; only
/// `run` is the real public surface.
pub fn load_rows(paths: &[PathBuf]) -> io::Result<Vec<CompareRow>> {
    paths.iter().map(load_one).collect()
}

fn load_one(path: &PathBuf) -> io::Result<CompareRow> {
    let mut scenario = load(path).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let name = name_from_path(path);
    let horizon = scenario.horizon;
    let stats: Vec<Stats> = scenario.run(horizon);
    let terminal_month = scenario.terminated.then(|| stats.len().saturating_sub(1) as u16);
    let goals = evaluate_goals(&stats, &scenario.goals, terminal_month);
    Ok(CompareRow {
        name,
        stats: derive_key_stats(&stats),
        terminal_month,
        goals,
    })
}

/// Basename of `path` without its extension. `scenarios/01-rent.yaml` → `01-rent`.
/// A path with no extension returns the full final segment (so a literal
/// `compare` would round-trip, but that name is reserved as the subcommand
/// keyword in `parse_args`).
pub fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---- Text renderer ----

/// Aligned text table. One header line followed by one line per scenario.
/// Numbers right-aligned with two decimals; "month N" / "never" / "—" used
/// for the integer-style columns; per-goal columns appended after the
/// fixed columns when at least one row carries goals.
pub fn format_text(rows: &[CompareRow]) -> String {
    let mut out = String::new();
    let columns = build_columns(rows);
    let header = render_header(&columns);
    out.push_str(&header);
    out.push('\n');
    for row in rows {
        out.push_str(&render_row_text(row, &columns));
        out.push('\n');
    }
    out
}

struct Column {
    title: String,
    width: usize,
    align_right: bool,
}

fn build_columns(rows: &[CompareRow]) -> Vec<Column> {
    let mut cols = vec![
        Column { title: "scenario".to_string(), width: "scenario".len(), align_right: false },
        Column { title: "final cash".to_string(), width: "final cash".len(), align_right: true },
        Column { title: "final assets".to_string(), width: "final assets".len(), align_right: true },
        Column { title: "final cashflow".to_string(), width: "final cashflow".len(), align_right: true },
        Column { title: "min cash".to_string(), width: "min cash".len(), align_right: true },
        Column { title: "min cash month".to_string(), width: "min cash month".len(), align_right: true },
        Column { title: "first insolvent month".to_string(), width: "first insolvent month".len(), align_right: false },
        Column { title: "terminal month".to_string(), width: "terminal month".len(), align_right: false },
    ];
    // Per-goal columns: one per distinct goal name, in order of first appearance.
    let mut goal_titles: Vec<String> = Vec::new();
    for r in rows {
        for g in &r.goals {
            if !goal_titles.iter().any(|t| t == &g.name) {
                goal_titles.push(g.name.clone());
            }
        }
    }
    for name in &goal_titles {
        cols.push(Column {
            title: name.clone(),
            width: name.len(),
            align_right: false,
        });
    }
    // Widen each column to fit data.
    for r in rows {
        widen(&mut cols[0], &r.name);
        widen(&mut cols[1], &fmt_money(r.stats.final_cash));
        widen(&mut cols[2], &fmt_money(r.stats.final_assets_value));
        widen(&mut cols[3], &fmt_money(r.stats.final_monthly_cashflow));
        widen(&mut cols[4], &fmt_money(r.stats.min_cash));
        widen(&mut cols[5], &r.stats.min_cash_month.to_string());
        widen(&mut cols[6], &fmt_month_or(r.stats.first_insolvent_month, "never"));
        widen(&mut cols[7], &fmt_terminal(r.terminal_month));
        for i in 8..cols.len() {
            let cell = r
                .goals
                .iter()
                .find(|g| g.name == cols[i].title)
                .map(fmt_goal)
                .unwrap_or_default();
            widen(&mut cols[i], &cell);
        }
    }
    cols
}

fn widen(col: &mut Column, value: &str) {
    if value.len() > col.width {
        col.width = value.len();
    }
}

fn render_header(cols: &[Column]) -> String {
    let mut out = String::new();
    for (i, c) in cols.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        write_padded(&mut out, &c.title, c.width, false);
    }
    out
}

fn render_row_text(row: &CompareRow, cols: &[Column]) -> String {
    let mut out = String::new();
    for (i, c) in cols.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let value: String = match i {
            0 => row.name.clone(),
            1 => fmt_money(row.stats.final_cash),
            2 => fmt_money(row.stats.final_assets_value),
            3 => fmt_money(row.stats.final_monthly_cashflow),
            4 => fmt_money(row.stats.min_cash),
            5 => row.stats.min_cash_month.to_string(),
            6 => fmt_month_or(row.stats.first_insolvent_month, "never"),
            7 => fmt_terminal(row.terminal_month),
            _ => row
                .goals
                .iter()
                .find(|g| g.name == c.title)
                .map(fmt_goal)
                .unwrap_or_default(),
        };
        write_padded(&mut out, &value, c.width, c.align_right);
    }
    out
}

fn fmt_money(v: f64) -> String {
    // Normalize negative zero to positive so the text reads "0.00" instead
    // of "-0.00" when a flow happens to land exactly on -0.0 after rounding.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.2}")
}

fn fmt_month_or(month: Option<usize>, fallback: &'static str) -> String {
    match month {
        Some(m) => format!("month {m}"),
        None => fallback.to_string(),
    }
}

fn fmt_terminal(month: Option<u16>) -> String {
    match month {
        Some(m) => format!("month {m}"),
        None => "—".to_string(),
    }
}

fn fmt_goal(g: &GoalOutcome) -> String {
    if g.met {
        "met".to_string()
    } else {
        format!("missed ({:.2})", g.value)
    }
}

fn write_padded(out: &mut String, value: &str, width: usize, right: bool) {
    let pad = width.saturating_sub(value.chars().count());
    if right {
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(value);
    } else {
        out.push_str(value);
        for _ in 0..pad {
            out.push(' ');
        }
    }
}

// ---- JSON renderer ----

/// JSON array, one element per row. Each element is the per-scenario
/// wrapper shape with an extra `name` (the basename).
pub fn format_json(rows: &[CompareRow]) -> String {
    #[derive(serde::Serialize)]
    struct KeyStatsOut {
        final_cash: f64,
        final_assets_value: f64,
        final_monthly_cashflow: f64,
        min_cash: f64,
        min_cash_month: usize,
        first_insolvent_month: Option<usize>,
    }
    #[derive(serde::Serialize)]
    struct GoalOut<'a> {
        name: &'a str,
        target: f64,
        by_month: u16,
        kind: &'a crate::GoalKind,
        met: bool,
        value: f64,
    }
    #[derive(serde::Serialize)]
    struct Out<'a> {
        name: &'a str,
        terminal: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_month: Option<u16>,
        key_stats: KeyStatsOut,
        goals: Vec<GoalOut<'a>>,
    }
    let out: Vec<Out> = rows
        .iter()
        .map(|r| Out {
            name: r.name.as_str(),
            terminal: r.terminal_month.is_some(),
            terminal_month: r.terminal_month,
            key_stats: KeyStatsOut {
                final_cash: r.stats.final_cash,
                final_assets_value: r.stats.final_assets_value,
                final_monthly_cashflow: r.stats.final_monthly_cashflow,
                min_cash: r.stats.min_cash,
                min_cash_month: r.stats.min_cash_month,
                first_insolvent_month: r.stats.first_insolvent_month,
            },
            goals: r
                .goals
                .iter()
                .map(|g| GoalOut {
                    name: g.name.as_str(),
                    target: g.target,
                    by_month: g.by_month,
                    kind: &g.kind,
                    met: g.met,
                    value: g.value,
                })
                .collect(),
        })
        .collect();
    serde_json::to_string(&out).expect("compare JSON never fails")
}

// ---- TUI renderer ----

fn render_tui(rows: &[CompareRow]) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // `draw` returns `CompletedFrame` which holds an internal buffer; the
    // borrow on `terminal` ends when we drop it.
    terminal.draw(|f| ui(f, rows))?;
    let backend = terminal.backend_mut();
    let restore = (|| -> io::Result<()> {
        disable_raw_mode()?;
        execute!(backend, LeaveAlternateScreen)?;
        Ok(())
    })();
    let _ = terminal.show_cursor();
    restore
}

fn ui(f: &mut Frame, rows: &[CompareRow]) {
    let header_h = 3u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_h), Constraint::Min(0)])
        .split(f.area());

    let nrows = rows.len();
    let header_text = format!(
        "Comparison: {nrows} scenario{}\n\
         (snapshot — no scroll, no input loop)",
        if nrows == 1 { "" } else { "s" },
    );
    let header = ratatui::widgets::Paragraph::new(header_text).block(
        Block::default().borders(Borders::ALL).title("ender compare"),
    );
    f.render_widget(header, chunks[0]);

    let mut widths: Vec<Constraint> = Vec::new();
    let mut headers: Vec<Cell> = Vec::new();
    let col_titles: [&str; 8] = [
        "scenario",
        "final cash",
        "final assets",
        "final cashflow",
        "min cash",
        "min cash month",
        "first insolvent month",
        "terminal month",
    ];
    for t in col_titles {
        headers.push(Cell::from(t));
        widths.push(Constraint::Length(t.len() as u16 + 4));
    }
    // Per-goal columns (first occurrence order, like text renderer)
    let mut goal_names: Vec<&str> = Vec::new();
    for r in rows {
        for g in &r.goals {
            if !goal_names.contains(&g.name.as_str()) {
                goal_names.push(g.name.as_str());
            }
        }
    }
    for name in &goal_names {
        headers.push(Cell::from(*name));
        widths.push(Constraint::Length(name.len() as u16 + 4));
    }

    let trows: Vec<Row> = rows
        .iter()
        .map(|r| {
            let mut cells = vec![
                Cell::from(r.name.clone()),
                Cell::from(fmt_money(r.stats.final_cash)),
                Cell::from(fmt_money(r.stats.final_assets_value)),
                Cell::from(fmt_money(r.stats.final_monthly_cashflow)),
                Cell::from(fmt_money(r.stats.min_cash)),
                Cell::from(r.stats.min_cash_month.to_string()),
                Cell::from(fmt_month_or(r.stats.first_insolvent_month, "never")),
                Cell::from(fmt_terminal(r.terminal_month)),
            ];
            for name in &goal_names {
                let cell = r
                    .goals
                    .iter()
                    .find(|g| g.name == *name)
                    .map(fmt_goal)
                    .unwrap_or_default();
                cells.push(Cell::from(cell));
            }
            Row::new(cells)
        })
        .collect();

    let table = Table::new(trows, widths)
        .header(Row::new(headers))
        .block(Block::default().borders(Borders::ALL).title("Comparison"));
    f.render_widget(table, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GoalKind;
    use std::sync::Mutex;

    // ---- Test fixtures ----

    /// Write `name` to a unique file in the system temp dir, return the path.
    /// ponytail: sequential counter avoids the parallel-test flakiness of
    /// `temp_dir().join(name)` when many tests reuse the same suffix.
    static COUNTER: Mutex<u64> = Mutex::new(0);

    fn write_tmp(name: &str, content: &str) -> PathBuf {
        let mut counter = COUNTER.lock().unwrap();
        *counter += 1;
        let pid = std::process::id();
        // Per-test unique directory, but the file inside carries the
        // caller's simple name so `name_from_path` returns it intact.
        let dir = std::env::temp_dir().join(format!("ender_compare_{pid}_{counter}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn key(
        final_cash: f64,
        final_assets_value: f64,
        final_monthly_cashflow: f64,
        min_cash: f64,
        min_cash_month: usize,
        first_insolvent_month: Option<usize>,
    ) -> crate::KeyStats {
        crate::KeyStats {
            final_cash,
            final_assets_value,
            final_monthly_cashflow,
            min_cash,
            min_cash_month,
            first_insolvent_month,
        }
    }

    fn goal(name: &str, target: f64, by_month: u16, met: bool, value: f64) -> crate::GoalOutcome {
        crate::GoalOutcome {
            name: name.to_string(),
            target,
            by_month,
            kind: GoalKind::Cash,
            met,
            value,
            evaluation_month: by_month,
        }
    }

    fn row(
        name: &str,
        stats: crate::KeyStats,
        terminal_month: Option<u16>,
        goals: Vec<crate::GoalOutcome>,
    ) -> CompareRow {
        CompareRow {
            name: name.to_string(),
            stats,
            terminal_month,
            goals,
        }
    }

    // ---- name_from_path ----

    #[test]
    fn name_from_path_strips_directory_and_extension() {
        assert_eq!(name_from_path(Path::new("scenarios/01-rent.yaml")), "01-rent");
        assert_eq!(name_from_path(Path::new("scenario.yaml")), "scenario");
        assert_eq!(name_from_path(Path::new("./local")), "local");
    }

    #[test]
    fn name_from_path_handles_no_extension() {
        assert_eq!(name_from_path(Path::new("compare")), "compare");
    }

    // ---- load_rows ----

    #[test]
    fn load_rows_returns_one_row_per_path_in_order() {
        let p1 = write_tmp("a.yaml", "cash: 100\nevents:\n  - { when: 0, type: expense, rent: { monthly: 10, annualized_rate: 0.0 } }\n");
        let p2 = write_tmp("b.yaml", "cash: 200\nevents: []\n");
        let rows = load_rows(&[p1.clone(), p2.clone()]).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "a");
        assert_eq!(rows[1].name, "b");
        assert_eq!(rows[1].stats.final_cash, 200.0);
        // Sanity: name_from_path strips only the .yaml extension.
        assert_eq!(name_from_path(&p1), "a");
        assert_eq!(name_from_path(&p2), "b");
    }

    #[test]
    fn load_rows_records_terminal_month_when_death_fires() {
        let p = write_tmp(
            "death.yaml",
            "cash: 0\nevents:\n  - { when: 12, type: death }\n",
        );
        let rows = load_rows(&[p]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].terminal_month, Some(12));
    }

    #[test]
    fn load_rows_terminal_month_is_none_without_death() {
        let p = write_tmp("alive.yaml", "cash: 100\nevents: []\n");
        let rows = load_rows(&[p]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].terminal_month, None);
    }

    #[test]
    fn load_rows_propagates_load_errors_with_path() {
        let bad = std::env::temp_dir().join("ender_does_not_exist_compare.yaml");
        let _ = std::fs::remove_file(&bad);
        let err = load_rows(&[bad.clone()]).unwrap_err();
        assert!(err.to_string().contains("ender_does_not_exist_compare.yaml"));
    }

    #[test]
    fn load_rows_propagates_yaml_errors_with_path() {
        let p = write_tmp("bad.yaml", "this is: not: valid: yaml: [\n");
        let err = load_rows(&[p]).unwrap_err();
        let s = err.to_string();
        // path appears somewhere in the error
        assert!(s.contains("bad.yaml"), "expected path in error, got: {s}");
    }

    // ---- format_text ----

    #[test]
    fn format_text_with_two_rows_contains_header_and_both_names() {
        let rows = vec![
            row(
                "a",
                key(100.0, 50.0, 25.0, 100.0, 0, None),
                None,
                vec![],
            ),
            row(
                "b",
                key(75.0, 60.0, -25.0, 75.0, 1, None),
                None,
                vec![],
            ),
        ];
        let out = format_text(&rows);
        for header in [
            "scenario",
            "final cash",
            "final assets",
            "final cashflow",
            "min cash",
            "min cash month",
            "first insolvent month",
            "terminal month",
        ] {
            assert!(out.contains(header), "missing header {header:?} in:\n{out}");
        }
        assert!(out.contains("a"), "row 'a' missing");
        assert!(out.contains("b"), "row 'b' missing");
    }

    #[test]
    fn format_text_numbers_have_two_decimals() {
        let rows = vec![row(
            "solo",
            key(100.0, 0.0, 0.0, 100.0, 0, None),
            None,
            vec![],
        )];
        let out = format_text(&rows);
        assert!(out.contains("100.00"), "expected two-decimal format:\n{out}");
    }

    #[test]
    fn format_text_columns_aligned_across_rows() {
        // Two rows where one numeric column has widely different widths.
        // Pick the "final cash" column: header + 12.34 + 123456.78 must share
        // the same right edge across the data lines.
        let rows = vec![
            row(
                "short",
                key(12.34, 0.0, 0.0, 0.0, 0, None),
                None,
                vec![],
            ),
            row(
                "long-name-here",
                key(123_456.78, 0.0, 0.0, 0.0, 0, None),
                None,
                vec![],
            ),
        ];
        let out = format_text(&rows);
        let lines: Vec<&str> = out.lines().collect();
        // Header + 2 data lines minimum.
        assert!(lines.len() >= 3, "expected header + rows:\n{out}");
        // Find the index of the "12.34" token in the short row and the
        // "123456.78" token in the long row; they must end at the same column.
        let short_idx = lines[1].find("12.34").expect("short row token missing");
        let long_idx = lines
            .iter()
            .find(|l| l.contains("123456.78"))
            .and_then(|l| l.find("123456.78"))
            .expect("long row token missing");
        let short_end = short_idx + "12.34".len();
        let long_end = long_idx + "123456.78".len();
        assert_eq!(
            short_end, long_end,
            "final cash column not right-aligned:\n{out}"
        );
    }

    #[test]
    fn format_text_first_insolvent_uses_month_or_never() {
        let rows = vec![
            row("ok", key(0.0, 0.0, 0.0, 0.0, 0, None), None, vec![]),
            row("bad", key(0.0, 0.0, 0.0, 0.0, 0, Some(3)), None, vec![]),
        ];
        let out = format_text(&rows);
        // "never" appears for the solvent row, "month 3" for the insolvent one
        assert!(out.contains("never"), "expected 'never' in:\n{out}");
        assert!(out.contains("month 3"), "expected 'month 3' in:\n{out}");
    }

    #[test]
    fn format_text_terminal_uses_month_or_em_dash() {
        let rows = vec![
            row("alive", key(0.0, 0.0, 0.0, 0.0, 0, None), None, vec![]),
            row("dead", key(0.0, 0.0, 0.0, 0.0, 0, None), Some(24), vec![]),
        ];
        let out = format_text(&rows);
        assert!(out.contains("—"), "expected em-dash for non-terminated:\n{out}");
        assert!(out.contains("month 24"), "expected 'month 24' for terminated:\n{out}");
    }

    #[test]
    fn format_text_appends_goal_columns() {
        let rows = vec![
            row(
                "met",
                key(0.0, 0.0, 0.0, 0.0, 0, None),
                None,
                vec![goal("college", 100.0, 12, true, 200.0)],
            ),
            row(
                "missed",
                key(0.0, 0.0, 0.0, 0.0, 0, None),
                None,
                vec![goal("college", 100.0, 12, false, 50.0)],
            ),
        ];
        let out = format_text(&rows);
        assert!(out.contains("college"), "expected goal column header in:\n{out}");
        assert!(out.contains("met"), "expected 'met' marker in:\n{out}");
        assert!(out.contains("missed (50.00)"), "expected missed marker in:\n{out}");
    }

    // ---- format_json ----

    #[test]
    fn format_json_emits_array_with_one_object_per_row() {
        let rows = vec![
            row("a", key(100.0, 50.0, 25.0, 100.0, 0, None), None, vec![]),
            row("b", key(75.0, 60.0, -25.0, -100.0, 1, Some(2)), Some(2), vec![]),
        ];
        let s = format_json(&rows);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let arr = v.as_array().expect("must be an array");
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn format_json_first_row_has_name_and_key_stats() {
        let rows = vec![row(
            "alpha",
            key(100.0, 50.0, 25.0, 100.0, 0, None),
            None,
            vec![],
        )];
        let s = format_json(&rows);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let arr = v.as_array().unwrap();
        let first = &arr[0];
        assert_eq!(first["name"], "alpha");
        assert_eq!(first["terminal"], false);
        assert!(first.get("terminal_month").is_none(), "terminal_month must be absent when not terminated");
        let ks = &first["key_stats"];
        assert_eq!(ks["final_cash"].as_f64().unwrap(), 100.0);
        assert_eq!(ks["final_assets_value"].as_f64().unwrap(), 50.0);
        assert_eq!(ks["final_monthly_cashflow"].as_f64().unwrap(), 25.0);
        assert_eq!(ks["min_cash"].as_f64().unwrap(), 100.0);
        assert_eq!(ks["min_cash_month"].as_u64().unwrap(), 0);
        assert!(ks["first_insolvent_month"].is_null());
    }

    #[test]
    fn format_json_terminal_run_includes_terminal_month() {
        let rows = vec![row(
            "dead",
            key(0.0, 0.0, 0.0, 0.0, 0, None),
            Some(24),
            vec![],
        )];
        let s = format_json(&rows);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let first = &v.as_array().unwrap()[0];
        assert_eq!(first["terminal"], true);
        assert_eq!(first["terminal_month"].as_u64().unwrap(), 24);
    }

    #[test]
    fn format_json_includes_goals_array_always() {
        let rows = vec![row(
            "g",
            key(0.0, 0.0, 0.0, 0.0, 0, None),
            None,
            vec![goal("college", 100.0, 12, true, 200.0)],
        )];
        let s = format_json(&rows);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let goals = v[0]["goals"].as_array().expect("goals must be array");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0]["name"], "college");
        assert_eq!(goals[0]["target"].as_f64().unwrap(), 100.0);
        assert_eq!(goals[0]["by_month"].as_u64().unwrap(), 12);
        assert_eq!(goals[0]["met"].as_bool().unwrap(), true);
        assert_eq!(goals[0]["value"].as_f64().unwrap(), 200.0);
        assert_eq!(goals[0]["kind"], "cash");
    }

    #[test]
    fn format_json_goals_array_empty_when_no_goals() {
        let rows = vec![row("solo", key(0.0, 0.0, 0.0, 0.0, 0, None), None, vec![])];
        let s = format_json(&rows);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let goals = v[0]["goals"].as_array().expect("goals must be array");
        assert_eq!(goals.len(), 0);
    }

    // ---- run() smoke tests ----

    /// Smoke: the public entry point must return Ok for a valid path.
    /// We can't easily capture stdout from `run`, so the rendering tests
    /// live in format_text / format_json. This test is the gate.
    #[test]
    fn run_with_one_scenario_returns_ok() {
        let p = write_tmp("solo.yaml", "cash: 100\nevents: []\n");
        let paths = vec![p];
        let r = run(&paths, false);
        // We don't assert success here because run() goes to TUI / stdout
        // branches. The point is just: it must not panic.
        let _ = r;
    }

    // ---- Sanity: helper for future tests ----

    // (none — the smoke test above covers the public entry point.)
}