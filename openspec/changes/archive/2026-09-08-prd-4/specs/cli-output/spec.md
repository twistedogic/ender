## MODIFIED Requirements

### Requirement: `--json` emits the per-month stats series
When `--json` is passed, the CLI SHALL write exactly one JSON document to stdout: a compact array with one object per simulated month, in simulation order, each carrying `month` (0-based index of the month), `cash`, `assets_value`, and `monthly_cashflow` as numbers. The document SHALL be the only stdout output in this mode, and SHALL be parseable by a standard JSON parser. Without `--json`: when stdout is a terminal, the interactive TUI SHALL be shown instead (capability `tui-display`); when stdout is not a terminal, the human-readable summary (and insolvency line, when applicable) SHALL be printed byte-for-byte as before this change.

#### Scenario: Series shape and indexing
- **WHEN** a scenario is run for 12 months with `--json`
- **THEN** stdout parses as a JSON array of 13 objects whose `month` fields are 0 through 12 in order, each carrying numeric `cash`, `assets_value`, and `monthly_cashflow`

#### Scenario: JSON matches the text result
- **WHEN** the same scenario is run with `--json` and without `--json` into a pipe
- **THEN** the last array element's `cash`, `assets_value`, and `monthly_cashflow` equal the values in the human summary line

#### Scenario: Insolvency is derivable, not duplicated
- **WHEN** a scenario goes insolvent and is run with `--json`
- **THEN** the array's first element with `cash` below zero is the same month the text mode reports as insolvent, and the JSON contains no separate insolvency field

#### Scenario: Text mode unchanged
- **WHEN** a scenario is run without `--json` and stdout is not a terminal (pipe or redirect)
- **THEN** stdout is byte-for-byte the pre-TUI output: the summary line, plus the insolvent line when one exists

#### Scenario: Terminal gets the TUI
- **WHEN** a scenario is run without `--json` and stdout is a terminal
- **THEN** no text summary is printed; the interactive TUI renders per the `tui-display` capability
