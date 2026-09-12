# Proposal: add-multi-scenario-comparison

## Why

ender runs one scenario at a time. To compare two scenarios, the
user must run each separately, capture the text summary (or JSON),
and diff by hand. The `financial-plan` skill's Step 5 is literally
"Scenario Modeling — Run key scenarios" with a "Probability of
Success | Portfolio at 90 | Notes" comparison table. ender has no
facility to produce that table.

A `compare` subcommand that takes N scenarios and emits a side-by-side
key-stats comparison unlocks the workflow: rent vs buy, base case vs
+20% spending, retire at 60 vs 65. The user's existing scenarios
become the rows; the run output becomes the diff.

## What Changes

- Add a new CLI subcommand `compare` that takes one or more scenario
  paths as positional arguments. The first non-flag argument SHALL
  be `compare`; the rest SHALL be paths.
- The subcommand SHALL load and simulate each scenario in sequence,
  collect key statistics for each, and render a comparison table:
  - **TTY**: a ratatui table with one row per scenario and one column
    per statistic (scenario name, final cash, final assets, min cash,
    min cash month, first insolvent month, terminal month when death-
    terminated, each goal's outcome).
  - **non-TTY**: aligned text with the same rows and columns. The
    existing 3-line summary per scenario is replaced by a single
    comparison table.
- `--json` SHALL apply: instead of a text or TUI table, emit a JSON
  array where each element is one scenario's key-stats summary
  (including any `goals` array from `add-goal-tracking` and any
  `terminal_month` from `add-insurance-modeling`).
- Unknown flags after `compare` SHALL fail with the same error format
  as today; missing scenario paths SHALL fail with `usage: ender
  compare <path>...`.
- The existing single-scenario CLI behavior is preserved: `ender
  scenario.yaml` and `ender --json scenario.yaml` continue to work
  exactly as today.

## Capabilities

### New Capabilities

- `multi-scenario-comparison`: the CLI can take N scenarios and
  produce a side-by-side comparison of their key statistics.

### Modified Capabilities

- `cli-output`: extend the CLI to accept a `compare` subcommand and
  dispatch on the first positional argument; add the new output
  format for `compare` (text table in non-TTY, ratatui table in TTY,
  JSON array with `--json`).

## Impact

- `src/main.rs`: dispatch on `args[0] == "compare"` in `main()`;
  delegate to a new `compare::run(paths, json)` function. The
  function loads each scenario, runs it, and renders the table.
- `src/compare.rs` (new): the comparison renderer. Holds a small
  `CompareRow { name, final_cash, final_assets, min_cash,
  min_cash_month, first_insolvent_month, terminal_month, goals }`
  struct and renders either a ratatui table or aligned text.
- `key_stats` gains the optional fields the comparison uses
  (`terminal_month`, `goals_outcomes`). Both new fields are
  already plumbed by `add-insurance-modeling` and
  `add-goal-tracking`; this change consumes them.
- `parse_args` (or a new dispatch helper) routes on the first
  positional argument: `compare` → comparison path; anything else
  → existing single-scenario path.
- `skills/ender/SKILL.md`: add the `compare` subcommand to the
  CLI section with one worked example
  (`ender compare scenario-a.yaml scenario-b.yaml`).
- Tests: dispatch on `compare` vs scenario path; the comparison
  table renders correct values for N scenarios; `--json` emits a
  well-formed array; missing paths fail with usage error; TUI
  mode renders without crashing.