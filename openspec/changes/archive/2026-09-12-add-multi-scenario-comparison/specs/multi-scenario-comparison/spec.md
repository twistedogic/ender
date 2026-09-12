## ADDED Requirements

### Requirement: CLI accepts a `compare` subcommand
The CLI SHALL dispatch on the first non-flag positional argument.
When that argument equals the literal string `compare`, the CLI
SHALL treat the remaining non-flag positional arguments as a list
of scenario paths (one or more) and SHALL delegate to the
comparison renderer. When the first non-flag positional argument
is anything else, the CLI SHALL continue to behave as today
(single scenario path, defaulting to `scenario.yaml`). The
`--json` flag SHALL continue to apply to both modes.

#### Scenario: Compare with two scenarios
- **WHEN** the CLI is run as
  `ender compare scenario-a.yaml scenario-b.yaml`
- **THEN** it loads and simulates both scenarios, then renders a
  comparison table with one row per scenario.

#### Scenario: Compare with one scenario
- **WHEN** the CLI is run as `ender compare scenario.yaml`
- **THEN** it loads and simulates the single scenario and renders a
  one-row comparison table.

#### Scenario: Compare with no paths fails
- **WHEN** the CLI is run as `ender compare` with no paths
- **THEN** it prints `usage: ender compare <path>...` to stderr and
  exits with code 1; nothing is loaded.

#### Scenario: Single-scenario mode is unchanged
- **WHEN** the CLI is run as `ender scenario.yaml`
- **THEN** the first argument is the scenario path, not the
  subcommand; the simulation and output are byte-identical to the
  pre-change behavior.

### Requirement: Comparison renders a key-stats table
The comparison renderer SHALL emit a table with one row per
scenario and the following columns: `scenario` (the scenario's
basename, without extension), `final cash`, `final assets`,
`final cashflow`, `min cash`, `min cash month`, `first insolvent
month`, `terminal month` (the value `month N` when death-
terminated, `—` otherwise). When `add-goal-tracking` is in effect,
the renderer SHALL additionally append one column per declared
goal showing `met` or `missed (value)`.

The renderer SHALL format numbers with two decimal places and SHALL
format month indexes as integers. Insolvency (`first insolvent
month`) SHALL render as `month N` or `never`. The terminal-month
column SHALL render as `month N` when set and `—` otherwise.

#### Scenario: Text comparison table renders side-by-side
- **WHEN** `ender compare scenarios/a.yaml scenarios/b.yaml` is run
  with stdout not a terminal
- **THEN** stdout contains a single comparison table with rows for
  `a` and `b` and the columns above; columns line up in plain text.

#### Scenario: TTY comparison table renders side-by-side
- **WHEN** the same command is run with stdout a terminal
- **THEN** a ratatui table with the same rows and columns is
  rendered in the alternate screen; pressing any key exits the
  alternate screen and returns to the shell prompt (no interactive
  loop).

### Requirement: `--json` emits a per-scenario array
When `--json` is passed with `compare`, the CLI SHALL emit a JSON
array to stdout with one element per scenario. Each element SHALL
carry:

- `name` (string) — the scenario's basename without extension.
- `terminal` (boolean) — true when the run was terminated by a
  `death` event.
- `terminal_month` (integer, optional) — present when `terminal`
  is true; the integer month at which the run stopped.
- `key_stats` — an object with `final_cash`, `final_assets_value`,
  `final_monthly_cashflow`, `min_cash`, `min_cash_month`,
  `first_insolvent_month`.
- `goals` (array) — one entry per goal (empty array when no
  goals), each entry carrying `name`, `target`, `by_month`,
  `kind`, `met`, `value`.

The document SHALL be the only stdout output in this mode and
SHALL be parseable by a standard JSON parser. The wrapper from
`add-insurance-modeling` and the goals array from
`add-goal-tracking` are reused unchanged per element.

#### Scenario: JSON comparison output
- **WHEN** `ender compare --json scenarios/a.yaml scenarios/b.yaml`
  is run
- **THEN** stdout parses as a 2-element JSON array; each element
  has the keys above; the per-element structure matches the
  single-scenario `--json` wrapper (with `name` added).

### Requirement: Compare preserves --json flag position
The `--json` flag MAY appear before or after `compare`, identical
to how it works for the single-scenario mode. An unknown flag
(other than `--json`) anywhere on the command line SHALL fail with
the same error format as today.

#### Scenario: --json before compare
- **WHEN** the CLI is run as `ender --json compare a.yaml b.yaml`
- **THEN** it loads both scenarios and emits the JSON array output.

#### Scenario: --json after compare
- **WHEN** the CLI is run as `ender compare a.yaml b.yaml --json`
- **THEN** it loads both scenarios and emits the JSON array output.

#### Scenario: Unknown flag after compare fails
- **WHEN** the CLI is run as `ender compare a.yaml --yaml`
- **THEN** it prints an error naming the unknown flag to stderr
  and exits with code 1, matching the existing single-scenario
  unknown-flag error.

### Requirement: Compare errors do not enter the TUI
When the comparison renderer encounters a load or simulation
error for any scenario, the error SHALL be reported on stderr with
exit code 1 BEFORE any TUI mode is entered; the terminal SHALL be
left untouched. This matches the existing `tui-display` capability's
"Errors never enter the TUI" requirement.

#### Scenario: Bad scenario path in compare
- **WHEN** `ender compare scenarios/missing.yaml a.yaml` is run
- **THEN** the missing-file error is reported on stderr with exit
  code 1; no TUI is shown.