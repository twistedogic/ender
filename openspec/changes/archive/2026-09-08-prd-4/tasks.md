## 1. Key statistics helper

- [x] 1.1 Write failing tests for a pure `key_stats(&[Stats])` helper: final month's `cash`/`assets_value`/`monthly_cashflow`; minimum `cash` with its month (ties → first month); first insolvent month `Some(m)` vs `None`; empty-series behavior (`main` never passes one — `Vec<Stats>` of a 0-month run still has month 0). Confirm the failure mode is the missing function.
- [x] 1.2 Implement `key_stats` as one pass over the series (min tracking + first-negative scan, same `cash < 0.0` rule as the text path). Tests go green.

## 2. TUI module (TODO 7, rendering side)

- [x] 2.1 Add `ratatui` and `crossterm` to `Cargo.toml`.
- [x] 2.2 Implement `tui::run(&stats) -> io::Result<()>`: enter alternate screen + raw mode; layout = key-statistics block (`key_stats`) above a `Table` of all months (`month`, `cash`, `assets_value`, `monthly_cashflow`) with `TableState` scrolling; event loop redraws on key and terminal events. Teardown (leave alternate screen, disable raw mode) runs on every exit path — quit, error — before returning.
- [x] 2.3 Key handling: `q`/`Esc` → quit; `Up`/`k` and `Down`/`j` → scroll one row; `PageUp`/`PageDown` → scroll by the visible table height. Selection clamped to the series bounds.

## 3. Wiring (TODO 7, selection side)

- [x] 3.1 Write a failing test that the selection logic picks the text path for a non-terminal stdout (extract `is_tty: bool` or test the branch via the pure pieces; the `std::io::IsTerminal` check itself is exercised by running the binary).
- [x] 3.2 Wire `main`: without `--json`, branch on `std::io::stdout().is_terminal()` — terminal → `tui::run(&stats)` (propagate `io::Error` to exit 1); non-terminal → today's summary + insolvent lines byte-for-byte. `--json` branch untouched.
- [x] 3.3 Manual verification: `cargo run` in a terminal (TUI renders, keys scroll, `q`/`Esc` restore the shell, exit 0); `cargo run | cat` prints the old summary; `cargo run -- --json | jq .[-1].cash` unchanged; `cargo run -- missing.yaml` errors before any terminal setup.

## 4. Bookkeeping and verification

- [x] 4.1 All existing tests stay green (`cargo test`).
- [x] 4.2 Check off TODOS item 7 — the intent's last open item; the next loop pass should retire the loop.
- [x] 4.3 `openspec validate prd-4 --strict` passes.
