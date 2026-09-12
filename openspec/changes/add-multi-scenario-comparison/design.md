## Context

ender's CLI today is `ender [flags] [path]`, where the path defaults
to `scenario.yaml`. The single-scenario output formats are:

- `--json` — JSON array of per-month stats to stdout
- TTY — interactive ratatui table with key-statistics header
- non-TTY — 3-line human summary

There is no facility to run multiple scenarios in one invocation
and produce a comparison. The user runs each scenario separately and
diff's output by hand, or pipes multiple runs into shell tools, or
reads JSON from N invocations into a script. All three are workable
but none is what the `financial-plan` skill's Step 5 calls for —
"Run key scenarios" with a structured comparison table.

Adding a `compare` subcommand is the smallest change that gives the
user the workflow they need without disturbing the existing single-
scenario path. The simulation, key-stats helper, and per-month stats
are all already in place — the new code is just dispatch + table
rendering.

## Goals / Non-Goals

**Goals:**

- A `compare` subcommand that takes one or more scenario paths.
- A key-stats comparison table (TTY: ratatui; non-TTY: aligned text)
  with rows = scenarios and columns = key statistics.
- `--json` emits a JSON array with one element per scenario, each
  carrying the scenario's key stats, terminal month (when death-
  terminated), and goals outcomes.
- The existing single-scenario CLI behavior is preserved unchanged.
- An unknown flag after `compare` fails with the same error format
  as today's unknown-flag failure.
- Missing scenario paths fail with a clear usage error.

**Non-Goals:**

- Interactive TUI for the comparison (scrollable, paginated). The
  table is rendered as a single screen; for many scenarios (10+)
  the user can pipe to `--json` and read in jq. A scrollable
  comparison view is a future enhancement.
- Cross-scenario diffs (e.g., "scenario B has $X more cash than
  scenario A"). The table shows absolute values; users compute
  diffs in jq or a spreadsheet.
- Statistical comparison ("probability of success"). ender is
  deterministic; the comparison is absolute side-by-side.
- Subcommands other than `compare` (e.g., `validate`, `diff`). The
  CLI today has one mode; adding the second makes it two. Three
  is the right time to introduce a clap-style parser; two is
  premature.
- Combining scenarios (e.g., "merge scenario A's events with
  scenario B's"). The user composes scenarios by hand; the
  comparison is read-only.

## Decisions

### D1: Subcommand via first positional argument, not clap
`ender compare <paths...>` is matched when the first non-flag argument
  is `compare`. The dispatch is a single `if args[0] == "compare"`
  branch in `main()`; the existing `parse_args` is reused for the
  paths and `--json` flag.

*Alternative rejected*: pull in `clap` for subcommand parsing —
heavyweight dependency for two modes. The first-arg-match approach
is ~5 lines and matches the project's "stdlib and native platform
features first" preference.

### D2: New `compare.rs` module, reuse existing building blocks
`src/compare.rs` exports `run(paths: &[PathBuf], json: bool) ->
io::Result<()>`. Internally it loads each scenario (existing
`load`), simulates it (existing `run`), computes key stats (existing
`key_stats`, extended by `add-insurance-modeling` and
`add-goal-tracking`), and renders the table. No new simulation
machinery.

*Alternative rejected*: inlining the comparison in `main.rs` —
`main.rs` is already 1500+ lines; a separate module is the right
boundary.

### D3: TTY vs non-TTY dispatch matches existing single-scenario
  pattern
The TUI uses ratatui (existing); the text output uses `print!` with
alignment helpers (a small `format_table(rows) -> String` helper).
The TUI is a non-interactive render (no `q`/`Esc` loop, no input
handling) — a snapshot, not a viewer. Users who want to drill in
run the scenarios individually with the existing TUI.

*Alternative rejected*: an interactive TUI for the comparison
(scroll rows, drill into a scenario) — much more code, a third
ratatui widget. The static table is enough for the comparison
question ("which scenario wins?").

### D4: `--json` array shape mirrors per-scenario wrapper
The existing per-scenario `--json` shape (introduced in
`add-insurance-modeling`) is `{terminal, terminal_month?,
months[], goals[]}` with each `goal` entry `{name, target,
by_month, kind, met, value}`. The comparison `--json` is a JSON
array whose elements are the per-scenario wrapper objects, with
one extra top-level `name` field (the scenario's basename) so the
consumer can correlate rows.

*Alternative rejected*: a top-level wrapper around the array
(`{scenarios: [...]}`) — adds nesting the user has to peel when
piping to jq. A flat array of named rows is more jq-friendly.

## Risks / Trade-offs

- [First-positional dispatch breaks `ender scenario.yaml` if a user
  names a scenario file `compare.yaml`] → Acceptable: the file is
  loaded as a scenario, not a subcommand, because the dispatch is
  based on the exact string `compare` in `args[0]`, not on
  extension. A scenario file literally named `compare` (no
  extension) would be misinterpreted; documented in the skill.
- [No interactive TUI for the comparison] → Users with many
  scenarios pipe to `--json`; users with few read the static
  table.
- [Comparison doesn't show per-month deltas] → Out of scope; the
  per-month stats are still available via the existing single-
  scenario `--json` for any individual scenario.

## Migration Plan

Pure addition. New module, new dispatch branch, new output formats.
Existing single-scenario CLI is byte-identical when `compare` is
not the first arg. Rollback = remove the dispatch branch and the
module. No persisted state.

## Open Questions

None blocking. If a future change wants interactive comparison
(scrolling, drill-down), that's a separate capability. If a future
change wants `ender diff a.yaml b.yaml` for per-month deltas, the
same `compare.rs` module is the natural home.