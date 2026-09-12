## ADDED Requirements

### Requirement: Scenario declares a goals list

The scenario file SHALL accept an optional top-level `goals:` field
alongside `cash`, `events`, and `saving`. The field SHALL be a list
of goals; absent or empty SHALL be equivalent to no goals declared
and SHALL NOT change the run's text or TUI rendering relative to a
scenario without the field. Each goal SHALL carry four fields:
`name` (a non-empty string), `target` (a positive number),
`by_month` (a 0-based month index, SHALL be ≤ the scenario's
horizon — otherwise loading fails with an error naming the file),
and `kind` (`cash` or `net_worth`; default `cash`). Two goals SHALL
NOT share both `name` and `by_month` (duplicate goal identity);
duplicates SHALL fail to load with an error naming the file.

#### Scenario: Goals list parses

- **WHEN** a scenario declares
  ```
  goals:
    - { name: college, target: 200000, by_month: 240, kind: cash }
    - { name: retirement, target: 1000000, by_month: 360, kind: net_worth }
  ```
- **THEN** loading succeeds and the scenario carries two goals with
  the stated names, targets, by_months, and kinds.

#### Scenario: Default kind is cash

- **WHEN** a goal is declared without `kind`
- **THEN** its kind is `cash`.

#### Scenario: Goal past horizon fails to load

- **WHEN** a goal's `by_month` exceeds the scenario's horizon (360
  by default, or `end - start + 1`)
- **THEN** loading fails with an error naming the file and
  identifying the offending goal.

### Requirement: Goals are evaluated at by_month (or terminal month)

For each goal, the simulator SHALL determine the evaluation month:
the smaller of `by_month` and the terminal month (when the run was
terminated by a `death` event from `add-insurance-modeling` before
`by_month`). At the evaluation month, the simulator SHALL compute
the goal's value:

- For `kind: cash`: the `cash` field at that month.
- For `kind: net_worth`: `cash + assets_value` at that month.

The goal SHALL be recorded as `met` when the value is `≥ target`,
and `missed` otherwise. The recorded outcome SHALL include the
value, the evaluation month, and the original goal metadata
(`name`, `target`, `by_month`, `kind`).

#### Scenario: Cash goal met

- **WHEN** a scenario declares a goal `{name: college, target:
  200000, by_month: 240, kind: cash}` and at month 240 cash is
  250000
- **THEN** the goal is recorded as met with `value: 250000.0` and
  `evaluation_month: 240`.

#### Scenario: Cash goal missed with value reported

- **WHEN** a scenario declares the same goal and at month 240 cash
  is 150000
- **THEN** the goal is recorded as missed with `value: 150000.0`
  and `evaluation_month: 240`.

#### Scenario: NetWorth goal uses cash plus assets

- **WHEN** a scenario declares `{name: retirement, target:
  1000000, by_month: 360, kind: net_worth}` and at month 360 cash
  is 50000 and the scenario's assets_value is 1100000
- **THEN** the goal is recorded as met with `value: 1150000.0`
  and `evaluation_month: 360`.

#### Scenario: Goal past terminal month evaluates at terminal month

- **WHEN** a scenario with `{type: death, when: 24}` declares a
  goal `{name: college, target: 200000, by_month: 240}`
- **THEN** loading fails because by_month 240 exceeds the death-
  terminated horizon of 24; the user's intent is a scenario bug
  and the loader surfaces it rather than silently evaluating at
  the terminal month.

### Requirement: Text summary reports one line per goal

When `goals:` is non-empty, the text summary SHALL append one line
per goal, ordered by `by_month` ascending (and within the same
`by_month`, by YAML order):

- For a met goal: `goal <name> met (<value> at month <evaluation_month>)`.
- For a missed goal: `goal <name> missed (<value> at month <evaluation_month>, target <target>)`.

When `goals:` is absent or empty, the text summary SHALL be
byte-identical to a scenario without the field.

#### Scenario: Text summary reports goals in by_month order

- **WHEN** a scenario declares two goals, one at `by_month: 240`
  and one at `by_month: 360`, the 360 goal met and the 240 goal
  missed
- **THEN** the text summary appends the missed 240 line first
  (by_month 240, missed) then the met 360 line (by_month 360,
  met).

#### Scenario: Text summary omits goal lines when no goals

- **WHEN** a scenario declares no `goals:` field
- **THEN** the text summary is byte-identical to the pre-change
  output.

### Requirement: `--json` output includes a top-level goals array

The `--json` wrapper object (introduced in `add-insurance-modeling`)
SHALL include a `goals` field: a JSON array with one entry per goal
in `by_month` order. Each entry SHALL have the keys `name`, `target`
(number), `by_month` (integer), `kind` (`"cash"` or `"net_worth"`),
`met` (boolean), and `value` (number). When `goals:` is absent or
empty, the `goals` field SHALL be an empty array, NOT omitted.

#### Scenario: JSON reports per-goal outcomes

- **WHEN** a scenario with two goals is run with `--json`
- **THEN** the document parses as an object whose `goals` field is
  a 2-element array, each element carrying the keys above in order.

### Requirement: TUI header lists goals

The TUI header SHALL render one extra line per goal, in `by_month`
order, formatted identically to the text summary lines. The
`Paragraph` height constraint SHALL grow by the number of goals so
the table area is not compressed.

#### Scenario: TUI header lists goals

- **WHEN** a scenario with two goals is run in TTY
- **THEN** the TUI header displays two extra lines for the goals,
  and the table below renders unchanged.

### Requirement: `saving:` and `goals:` coexist

A scenario MAY declare both `saving:` and `goals:`. The simulator
SHALL evaluate both independently: `saving:` reports the first
breach month of the cash floor (existing behavior), and each goal
is reported per its own by_month. Both SHALL appear in the text
summary in the order `saving breach line` then `goal lines`.
`--json` SHALL include both `saving_breach_month` (the existing
field, if applicable) and `goals` (the new field).

#### Scenario: saving and goals both reported

- **WHEN** a scenario declares `saving: 50000` and a goal
  `{name: college, target: 200000, by_month: 240}`
- **THEN** the text summary includes both `saving target breached
  at month N` (when applicable) and the goal line(s); neither
  suppresses the other.