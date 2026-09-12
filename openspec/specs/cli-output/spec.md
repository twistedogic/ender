# cli-output Specification

## Purpose

Wire the CLI: accept a scenario path (single-scenario mode) or a `compare` subcommand with N scenario paths (multi-scenario mode), plus the `--json` flag, and write either the human summary, the JSON stats series, or a side-by-side comparison table to stdout.
## Requirements
### Requirement: CLI takes a scenario path and optional flags
The CLI SHALL accept an optional scenario file path as a positional argument (defaulting to `scenario.yaml` when absent) and an optional `--json` flag, in either order relative to the path. Any other argument beginning with `-` SHALL fail with an error message on stderr and exit code 1. Loading errors SHALL be reported on stderr with exit code 1 regardless of `--json`.

When the first non-flag positional argument equals the literal `compare`, the CLI SHALL treat the remaining positional arguments as a list of one or more scenario paths and delegate to the comparison renderer (capability `multi-scenario-comparison`); `--json` continues to apply in that mode. When the first non-flag positional argument is anything else (or absent), the CLI behaves as the single-scenario path described above. A scenario file literally named `compare` (no extension) would be misinterpreted as the subcommand; documented in the skill.

#### Scenario: Default invocation
- **WHEN** the CLI is run with no arguments
- **THEN** it loads `scenario.yaml` and prints the human summary to stdout, as before this change

#### Scenario: Flag after path
- **WHEN** the CLI is run as `ender scenario.yaml --json`
- **THEN** it loads `scenario.yaml` and prints the JSON series to stdout

#### Scenario: Flag before path
- **WHEN** the CLI is run as `ender --json scenario.yaml`
- **THEN** it loads `scenario.yaml` and prints the JSON series to stdout

#### Scenario: Unknown flag fails
- **WHEN** the CLI is run with an argument `--yaml` (any `-`-prefixed argument other than `--json`)
- **THEN** it prints an error naming the unknown flag to stderr and exits with code 1, loading nothing

#### Scenario: `compare` with two scenarios
- **WHEN** the CLI is run as `ender compare a.yaml b.yaml`
- **THEN** it routes to the comparison renderer (see capability `multi-scenario-comparison`); single-scenario loading is skipped.

#### Scenario: `compare` with no paths
- **WHEN** the CLI is run as `ender compare` (no paths)
- **THEN** it prints `usage: ender compare <path>...` to stderr and exits with code 1; nothing is loaded.

#### Scenario: Unknown flag after `compare`
- **WHEN** the CLI is run as `ender compare a.yaml --yaml`
- **THEN** it prints `unknown flag: --yaml` to stderr and exits with code 1, matching the single-scenario unknown-flag error format.

### Requirement: `--json` emits the per-month stats series
When `--json` is passed, the CLI SHALL write exactly one JSON document to stdout: a wrapper object with two fields, `months` and `goals`. `months` is a compact array with one object per simulated month, in simulation order, each carrying `month` (0-based index of the month), `cash`, `assets_value`, and `monthly_cashflow` as numbers. `goals` is an array of per-goal outcomes (see capability `goal-tracking`); it is always present and `[]` when the scenario declares no goals. The document SHALL be the only stdout output in this mode, and SHALL be parseable by a standard JSON parser. Without `--json`: when stdout is a terminal, the interactive TUI SHALL be shown instead (capability `tui-display`); when stdout is not a terminal, the human-readable summary (and insolvency line, when applicable) SHALL be printed byte-for-byte as before this change.

#### Scenario: Series shape and indexing
- **WHEN** a scenario is run for 12 months with `--json`
- **THEN** stdout parses as a JSON object whose `months` field is an array of 13 objects whose `month` fields are 0 through 12 in order, each carrying numeric `cash`, `assets_value`, and `monthly_cashflow`; the `goals` field is also present (an array, length 0 when no goals declared)

#### Scenario: JSON matches the text result
- **WHEN** the same scenario is run with `--json` and without `--json` into a pipe
- **THEN** the last element of `months` has `cash`, `assets_value`, and `monthly_cashflow` equal to the values in the human summary line

#### Scenario: Insolvency is derivable, not duplicated
- **WHEN** a scenario goes insolvent and is run with `--json`
- **THEN** the `months` array's first element with `cash` below zero is the same month the text mode reports as insolvent, and the JSON contains no separate insolvency field

#### Scenario: Text mode unchanged
- **WHEN** a scenario is run without `--json` and stdout is not a terminal (pipe or redirect)
- **THEN** stdout is byte-for-byte the pre-TUI output: the summary line, plus the insolvent line when one exists

#### Scenario: Terminal gets the TUI
- **WHEN** a scenario is run without `--json` and stdout is a terminal
- **THEN** no text summary is printed; the interactive TUI renders per the `tui-display` capability

