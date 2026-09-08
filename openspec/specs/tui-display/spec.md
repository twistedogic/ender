# tui-display Specification

## Purpose
Render the simulation as an interactive terminal user interface when the user has not asked for machine-readable output: a key-statistics header over a scrollable per-month table, driven by `q`/`Esc`, arrows/`j`/`k`, and PageUp/PageDown.
## Requirements
### Requirement: Interactive TUI renders the run when no `--json` is passed
When the CLI is invoked without `--json` and stdout is connected to a terminal, it SHALL launch an interactive terminal user interface (TUI) rendering the completed simulation instead of printing text to stdout. The TUI SHALL appear only after the scenario has loaded and the simulation has run; load and argument errors SHALL be reported on stderr with exit code 1 before any terminal mode is entered.

#### Scenario: Terminal invocation
- **WHEN** the CLI is run with no flags and stdout is a terminal
- **THEN** the screen switches to the TUI showing the run's statistics, and nothing is printed to stdout

#### Scenario: Errors never enter the TUI
- **WHEN** the CLI is run with a missing scenario file and stdout is a terminal
- **THEN** the error goes to stderr with exit code 1 and the terminal is left untouched

### Requirement: Key statistics of the run are displayed
The TUI SHALL display the key statistics of the series: the final month's `cash`, `assets_value`, and `monthly_cashflow`; the minimum `cash` over the series together with the month it occurs; and the first insolvent month (first month with `cash` below zero) when one exists. The insolvency rule SHALL be identical to the text mode's.

#### Scenario: Final values shown
- **WHEN** a scenario is run for 12 months in the TUI
- **THEN** the header shows month 12's `cash`, `assets_value`, and `monthly_cashflow`, equal to the values the text summary would print

#### Scenario: Minimum cash and insolvency shown
- **WHEN** a scenario's cash dips negative before recovering
- **THEN** the header shows the minimum cash with its month, and that same month as the first insolvent month

#### Scenario: Solvent run
- **WHEN** a scenario never goes insolvent
- **THEN** the header shows the minimum cash with its month and reports no insolvency

### Requirement: The monthly series is browsable
The TUI SHALL list the full per-month series — `month`, `cash`, `assets_value`, `monthly_cashflow` for every simulated month — as a scrollable table. `Up`/`Down` and `j`/`k` SHALL scroll one row; `PageUp`/`PageDown` SHALL scroll by a screen; the selection SHALL stay within the series.

#### Scenario: Every month is reachable
- **WHEN** a scenario is run for 12 months and the table is scrolled to the end
- **THEN** the last row shown is month 12, and scrolling cannot move past it or before month 0

### Requirement: The TUI exits cleanly
`q` or `Esc` SHALL quit the TUI, restore the terminal to its prior state (leaving raw mode and the alternate screen), and exit with code 0.

#### Scenario: Quit keys
- **WHEN** `q` or `Esc` is pressed
- **THEN** the TUI closes, the shell prompt returns to a sane terminal, and the process exits 0

