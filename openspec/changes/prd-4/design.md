## Context

`src/main.rs` (single file, ~1470 lines) runs 360 months from a YAML scenario and collects one `Stats { cash, assets_value, monthly_cashflow }` per month into a `Vec`. `main` already branches on the `--json` flag (prd-3): JSON mode prints the serialized series; the other branch prints a one-line summary of the final month plus the insolvent line when the series ever goes negative. `ratatui` is not yet a dependency. prd-3's design explicitly left the text output for the TUI iteration to replace.

## Goals / Non-Goals

**Goals:**

- TODO 7: an interactive ratatui terminal user interface (TUI) shown when `--json` is not passed, displaying key statistics and the full monthly series.
- `--json` output stays byte-for-byte identical; piped/redirected runs keep the text summary.

**Non-Goals:**

- Charts/graphs of the trajectory — a scrollable table shows every data point with less code; add a chart if a picture is ever asked for.
- Month filtering, field selection, or scenario editing inside the TUI — YAGNI.
- A `--no-tui` / `--tui` flag — automatic terminal detection covers both paths.
- Colors/themes configuration — ratatui's default styling suffices.

## Decisions

1. **Detect the terminal, don't add a flag.** `std::io::stdout().is_terminal()` (in std since 1.70) picks TUI vs. text. Zero new flags, `ender | less` and redirects keep working, and the unix convention (interactive defaults, piped output plain) falls out for free. Alternative: a `--tui` flag — rejected: the intent names no flag and auto-detection is less to document and parse.

2. **`ratatui` + `crossterm` as explicit dependencies.** The intent names ratatui; modern ratatui does not bundle a backend, so `crossterm` comes along. Two focused deps for the requested feature, nothing else.

3. **Key-statistics block + scrollable table, not a chart.** The information in the run is: three final values, the worst moment (minimum cash and its month), and insolvency. A header block derived by one pure pass over the series, plus ratatui's `Table` with `TableState` scrolling over all 361 rows. A `Chart` widget would add axes, bounds, and dataset code to visualize the same numbers. ponytail: table over chart; revisit if a visual trajectory is requested.

4. **One `tui::run(&stats) -> io::Result<()>` function owns the whole terminal lifecycle.** Enter alternate screen + raw mode, loop on events, restore on every exit path (quit key, and terminal teardown before propagating errors). `main` calls it only after a successful load and run, so no simulation error path ever touches the terminal.

5. **Derived statistics live in a pure, tested helper.** `key_stats(&[Stats])` returns final values, minimum cash with its month, and the first insolvent month (same rule as the text path: first `cash < 0`). This is the logic that can silently lie; it gets failing-first tests. Rendering itself is not unit-tested — it either draws or visibly doesn't; the binary run is the check. Alternative: computing stats inline in the render closure — rejected: untestable and duplicated against the text path.

6. **Keep the text summary as the non-terminal path rather than deleting it.** Deleting it would spray escape codes into pipes. It is already written, already pinned byte-for-byte by the cli-output spec, and costs nothing to keep behind the `is_terminal()` branch.

## Risks / Trade-offs

- Replacing the default terminal output with a TUI changes what a human script driving a pseudo-terminal sees. Real pipes and redirects are unaffected; anyone automating against a pty should use `--json`, which is the stable machine contract.
- A minimal keyset (`q`/`Esc`, arrows/`j`/`k`, PageUp/PageDown) — no search, no goto-month. Add keys when a user asks for them.
- No resize-aware recomputation beyond what ratatui redraw gives for free (the table reflows on the next draw; the event loop redraws on terminal events).
