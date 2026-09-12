## ADDED Requirements

### Requirement: Scenario declares a death event that ends the simulation at its firing month

The scenario file SHALL accept a new event type `death` alongside
`job`, `expense`, `tuition`, `end`, and the other existing
variants. The event SHALL carry no body. When the event fires
(at its `when` month), the system SHALL apply any other events
scheduled for that month, settle the cashflows (including any
scheduled `one_off_income` payouts), and SHALL then stop the
simulation: no further months are simulated, no further events
fire, and the run reports the firing month as the terminal month.
A scenario that does not declare a `death` event SHALL run to the
configured horizon (default 360, or `end - start + 1` months)
exactly as before.

#### Scenario: Death event terminates the run

- **WHEN** a scenario declares `{ type: death, when: 60 }` and a
  horizon of 360 months
- **THEN** the simulator runs 61 simulated months (months 0..=60)
  and reports month 60 as the terminal month; the run output
  identifies the run as terminated.

#### Scenario: Death with no other events produces a single-month run

- **WHEN** a scenario declares only `{ type: death, when: 0 }`
- **THEN** the simulator runs exactly 1 simulated month (month 0)
  and reports it as the terminal month; no other events fire.

#### Scenario: Scheduled payout fires at the death month

- **WHEN** a scenario declares
  `{ type: death, when: 1080 }` and a
  `{ type: one_off_income, when: 1080, one_off: {amount: 1000000} }`
- **THEN** month 1080's `monthly_cashflow` includes the +1000000
  payout AND the run terminates after month 1080; the terminal
  cash reflects the post-payout balance.

#### Scenario: Death before any month ends the run with zero events fired

- **WHEN** a scenario declares `{ type: death, when: 0 }` before
  any other event
- **THEN** the simulator runs 1 month (month 0); no other events
  are processed; the run reports month 0 as the terminal month.

### Requirement: Run output reports terminal month in text and TUI modes

The text summary SHALL append a single line `terminal at month N
(death event)` when the run was terminated by a death event, and
SHALL omit the line otherwise. The TUI header SHALL display the
same information in a single extra line. The TUI scrollable table
SHALL list every simulated month up to and including the terminal
month and SHALL NOT list months beyond it.

#### Scenario: Text summary reports termination

- **WHEN** a scenario with `{ type: death, when: 24 }` runs to
  completion
- **THEN** the text summary ends with `terminal at month 24
  (death event)` on its own line; the final-month cash + assets
  line above it reports month 24's values.

#### Scenario: Text summary omits termination when no death event

- **WHEN** a scenario with no `death` event runs to its horizon
- **THEN** the text summary is byte-identical to the pre-change
  output (no termination line added).

### Requirement: `--json` output wraps the array with a `terminal` field

The `--json` output SHALL wrap the existing per-month stats array
in a JSON object with at least two top-level fields: `terminal`
(`true` when the run was cut short by a death event, `false`
otherwise) and `months` (the existing array, unchanged in shape).
When `terminal` is `true`, an additional `terminal_month` field
SHALL be present with the integer month index at which the run
stopped; when `terminal` is `false`, `terminal_month` SHALL be
absent.

#### Scenario: JSON output reports termination

- **WHEN** a scenario with `{ type: death, when: 24 }` runs with
  `--json`
- **THEN** the document parses as an object with
  `terminal: true`, `terminal_month: 24`, and `months` an array
  of 25 per-month objects (`month` 0 through 24).

#### Scenario: JSON output unchanged shape when no death event

- **WHEN** a scenario with no `death` event runs with `--json`
- **THEN** the document parses as an object with
  `terminal: false` and `months` an array of horizon+1 per-month
  objects; no `terminal_month` field is present.

### Requirement: Death event is documented as part of insurance modeling

The skill SHALL include a "Risk management" section documenting
how to model life insurance and income-protection (disability)
using the existing primitives plus the `death` event. The
section SHALL include two worked examples that the user can run
end-to-end against a `scenarios/` YAML file:

1. **Life insurance**: a recurring premium expense from `when: 0`,
   a `one_off_income` of the policy's payout at the assumed death
   month, and a `death` event at the same month. The terminal
   cash after the payout is the estate's liquid value at death.

2. **Income protection / disability**: a recurring premium expense,
   a `pension:` (or `expense: {monthly: 0}`) replacement income
   starting at the disability month, and an `end: {id: main-job}`
   removing the salary. No `death` event — the run continues at
   a lower income level until the configured horizon.

#### Scenario: Worked life-insurance scenario runs end-to-end

- **WHEN** `scenarios/insurance-life.yaml` is executed with the
  skill's documented parameters
- **THEN** the run reports a terminal month equal to the death
  month in the YAML and the terminal cash reflects the
  pre-payout balance plus the policy payout, minus any expense
  premiums accrued to that point.

#### Scenario: Worked disability scenario runs end-to-end

- **WHEN** `scenarios/insurance-disability.yaml` is executed
  with the skill's documented parameters
- **THEN** the run reaches its configured end (no death event)
  and the final cash reflects a lower monthly income (salary
  removed, replacement pension in) for the post-disability months.