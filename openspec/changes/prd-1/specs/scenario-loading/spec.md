## MODIFIED Requirements

### Requirement: Scenario is defined in a YAML file
The system SHALL load a scenario from a YAML file containing starting `cash` and a list of `events`, where each event has a `when` (month index, 0-based) and a `type` discriminator. Supported event types SHALL include `job`, `expense`, `tuition`, `layoff`, `graduate`, `buy_home`, `buy_to_let`, `investment`, `end`, `one_off_expense`, and `one_off_income`.

#### Scenario: Minimal scenario
- **WHEN** a YAML file contains `cash: 20000` and an empty `events` list
- **THEN** loading it yields a `Scenario` with 20000.0 cash, no cashflows, no assets, and no events

#### Scenario: Full event vocabulary
- **WHEN** a YAML file uses each event type — `job`, `expense`, `tuition`, `buy_home`, `buy_to_let`, `investment`, `end`, `one_off_expense`, `one_off_income` — with their required fields
- **THEN** loading it yields a `Scenario` whose cashflow and asset lists contain the corresponding items, and whose event list contains the `end` event

### Requirement: Simulation state is not part of the file format
Fields that are runtime simulation state SHALL be defaulted during deserialization and SHALL NOT be required in the YAML file. Specifically, `Mortgage.paid` SHALL default to `0` and `OneOff.fired` SHALL default to `false`.

#### Scenario: Mortgage without paid counter
- **WHEN** a YAML mortgage specifies only `monthly` and `period`
- **THEN** loading succeeds and the mortgage behaves as if nothing has been paid yet

#### Scenario: One-off without fired flag
- **WHEN** a YAML one-off specifies only `amount`
- **THEN** loading succeeds and the one-off behaves as if it has not fired yet

### Requirement: Cashflow items may carry an id label
Every cashflow item, wherever it appears — added by a top-level event (`job`, `expense`, `tuition`, `one_off_expense`, `one_off_income`) or embedded in an asset (fund `investment`, property `rental`, property `mortgage`) — SHALL accept an optional string `id`. Items without an `id` SHALL load and behave exactly as before.

#### Scenario: Labeled salary loads
- **WHEN** a `job` event specifies `{ type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.03 } }`
- **THEN** loading succeeds and the resulting cashflow item carries id `partner`

#### Scenario: Unlabeled scenario is unchanged
- **WHEN** a scenario file with no `id` fields anywhere is loaded
- **THEN** loading succeeds and behavior is identical to before this change

## ADDED Requirements

### Requirement: One-off cashflows fire once and appear in the firing month's cashflow
A `one_off_expense` or `one_off_income` event SHALL add a cashflow item that contributes its `amount` to the monthly cashflow exactly once, in the month the event fires, and zero in every later month. The amount SHALL appear in that month's reported `monthly_cashflow` (unlike asset purchase prices, which are one-time deductions, not flows). The direction of the flow SHALL be determined by the event type — expense or income — using the absolute value of `amount`, so a negative `amount` SHALL behave identically to its positive counterpart.

#### Scenario: One-off expense lands once
- **WHEN** a `one_off_expense` event with `when: 3` and `one_off: { amount: 5000 }` fires in a scenario with cash 20000 and no other cashflows
- **THEN** month 3's monthly cashflow is −5000 and cash is 15000, and every later month's monthly cashflow is 0

#### Scenario: One-off income lands once
- **WHEN** a `one_off_income` event with `when: 6` and `one_off: { amount: 10000 }` fires in a scenario with cash 0 and no other cashflows
- **THEN** month 6's monthly cashflow is +10000 and cash is 10000, and every later month's monthly cashflow is 0

#### Scenario: Sign is taken from the event type
- **WHEN** a `one_off_expense` event specifies `one_off: { amount: -5000 }`
- **THEN** it loads and behaves exactly as `{ amount: 5000 }` — the flow is outgoing

#### Scenario: One-off may carry an id
- **WHEN** a `one_off_expense` event specifies an `id` and an `end` event with that id fires before the one-off's month
- **THEN** the one-off is removed and contributes nothing in its month
