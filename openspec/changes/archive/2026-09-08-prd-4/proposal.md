## Why

The intent's second concern — making the result readable — has a machine half but not a human half. `--json` (prd-3) unlocked piping, but the default human output is still one summary line: 361 months of trajectory compressed into three numbers. A human cannot see when cash dips lowest, when insolvency hits, or how assets evolve. The intent states it directly: "when `--json` is not passed, render an interactive TUI showing key stats over the simulation."

## Position in the improvement loop

Fourth iteration (`prd-4`) of the continuous improvement loop on `.see/intent.md` — the final open item.

- `prd-1` shipped TODOS 1–2: `one_off_expense` and `one_off_income` event types.
- `prd-2` shipped TODOS 3–4: `reserve_months` and reserve-aware payment precedence, with insolvency reporting.
- TODO 5 (HK salaries/property tax, net-of-MPF salary) shipped outside the loop via `2026-09-08-add-hk-tax`.
- `prd-3` shipped TODO 6: the `--json` flag and the stats-series contract, deliberately deferring the terminal user interface (TUI) to its own iteration.

This change ships TODO 7: the ratatui TUI shown when `--json` is not passed. After it applies and archives, every TODO in the intent is done and the loop retires on its next pass.

## What Changes

- Without `--json`, when stdout is connected to a terminal, `main` launches an interactive ratatui TUI instead of printing the text summary.
- The TUI shows the key statistics of the run — final-month cash, assets value, and monthly cashflow; the minimum cash over the series and the month it lands; the first insolvent month when one exists — plus the full per-month series (`month`, `cash`, `assets_value`, `monthly_cashflow`) as a scrollable table.
- Key handling: `q` or `Esc` quits; `Up`/`Down` (and `j`/`k`) scroll the series one row; `PageUp`/`PageDown` scroll by a screen. On exit the terminal state is restored.
- Without `--json` and with stdout not a terminal (a pipe or redirect), today's text summary is printed byte-for-byte — no escape codes into pipes, no new flag to learn.
- `--json` mode, argument parsing, and error handling (stderr, exit 1) are unchanged.

## Capabilities

### New Capabilities

- `tui-display`: interactive terminal rendering of the stats series — what is shown (key statistics, monthly table) and how it is driven (scroll, quit), entered when `--json` is absent and stdout is a terminal.

### Modified Capabilities

- `cli-output`: the "`--json` emits the per-month stats series" requirement is updated — its "without `--json`" clause no longer pins the text summary unconditionally; the text summary is now the non-terminal path, and a terminal gets the TUI (owned by `tui-display`).

## Impact

- `Cargo.toml`: adds `ratatui` and `crossterm` (named by the intent; ratatui needs an explicit backend).
- `src/main.rs`: `main`'s non-JSON branch gains a terminal check (`std::io::IsTerminal`) choosing between the TUI and the existing text summary; a small `tui` module (`run(&stats)` event loop plus rendering) and a pure `key_stats(&stats)` helper. No simulation changes.
- `scenario.yaml`: unchanged.
- `TODOS`: item 7 checked off — the last open item.
- Tests: failing-first unit tests for `key_stats` (final values, minimum cash and its month, insolvency detection) in `src/main.rs`; rendering itself is verified by running the binary.
