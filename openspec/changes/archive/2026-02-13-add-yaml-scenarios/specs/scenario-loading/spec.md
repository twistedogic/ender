# Spec: scenario-loading

## ADDED Requirements

### Requirement: Scenario is defined in a YAML file
The system SHALL load a scenario from a YAML file containing starting `cash` and a list of `events`, where each event has a `when` (month index, 0-based) and a `type` discriminator.

#### Scenario: Minimal scenario
- **WHEN** a YAML file contains `cash: 20000` and an empty `events` list
- **THEN** loading it yields a `Scenario` with 20000.0 cash, no cashflows, no assets, and no events

#### Scenario: Full event vocabulary
- **WHEN** a YAML file uses each event type — `job`, `expense`, `layoff`, `buy_home`, `buy_to_let`, `investment` — with their required fields
- **THEN** loading it yields a `Scenario` whose cashflow and asset lists contain the corresponding items

### Requirement: Simulation state is not part of the file format
Fields that are runtime simulation state SHALL be defaulted during deserialization and SHALL NOT be required in the YAML file. Specifically, `Mortgage.paid` SHALL default to `0`.

#### Scenario: Mortgage without paid counter
- **WHEN** a YAML mortgage specifies only `monthly` and `period`
- **THEN** loading succeeds and the mortgage behaves as if nothing has been paid yet

### Requirement: Events fire once
An event SHALL be consumed when it fires (the month in `when` is reached) and SHALL NOT fire again on later months.

#### Scenario: Event does not refire
- **WHEN** an `expense` event with `when: 3` is loaded and the scenario runs for 12 months
- **THEN** the expense cashflow is added exactly once, at month 3

### Requirement: Invalid files fail with a clear error
Loading a YAML file that is missing required fields, has unknown event types, or is not valid YAML SHALL return an error that identifies the file path and the reason.

#### Scenario: Unknown event type
- **WHEN** an event has `type: lottery_win`
- **THEN** loading fails with an error naming the file and the unknown variant

#### Scenario: Malformed YAML
- **WHEN** the file is not valid YAML
- **THEN** loading fails with an error naming the file and the parse failure
