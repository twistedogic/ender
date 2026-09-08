# cli-output Specification

## Purpose

Wire the CLI: accept a scenario path and the `--json` flag, and write either the human summary or a JSON stats series to stdout.
## Requirements
### Requirement: CLI takes a scenario path and optional flags
The CLI SHALL accept an optional scenario file path as a positional argument (defaulting to `scenario.yaml` when absent) and an optional `--json` flag, in either order relative to the path. Any other argument beginning with `-` SHALL fail with an error message on stderr and exit code 1. Loading errors SHALL be reported on stderr with exit code 1 regardless of `--json`.

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

### Requirement: `--json` emits the per-month stats series
When `--json` is passed, the CLI SHALL write exactly one JSON document to stdout: a compact array with one object per simulated month, in simulation order, each carrying `month` (0-based index of the month), `cash`, `assets_value`, and `monthly_cashflow` as numbers. The document SHALL be the only stdout output in this mode, and SHALL be parseable by a standard JSON parser. Without `--json`, the human-readable summary (and insolvency line, when applicable) SHALL be printed exactly as before this change.

#### Scenario: Series shape and indexing
- **WHEN** a scenario is run for 12 months with `--json`
- **THEN** stdout parses as a JSON array of 13 objects whose `month` fields are 0 through 12 in order, each carrying numeric `cash`, `assets_value`, and `monthly_cashflow`

#### Scenario: JSON matches the text result
- **WHEN** the same scenario is run with and without `--json`
- **THEN** the last array element's `cash`, `assets_value`, and `monthly_cashflow` equal the values in the human summary line

#### Scenario: Insolvency is derivable, not duplicated
- **WHEN** a scenario goes insolvent and is run with `--json`
- **THEN** the array's first element with `cash` below zero is the same month the text mode reports as insolvent, and the JSON contains no separate insolvency field

#### Scenario: Text mode unchanged
- **WHEN** a scenario is run without `--json`
- **THEN** stdout is byte-for-byte the pre-change output: the summary line, plus the insolvent line when one exists

