## MODIFIED Requirements

### Requirement: Scenario is defined in a YAML file
The system SHALL load a scenario from a YAML file containing starting `cash` and a list of `events`, where each event has a `when` (month index, 0-based) and a `type` discriminator. Supported event types SHALL include `job`, `expense`, `tuition`, `layoff`, `graduate`, `buy_home`, `buy_to_let`, `investment`, `end`, `one_off_expense`, `one_off_income`, and `death`.

#### Scenario: Full event vocabulary
- **WHEN** a YAML file uses each event type — `job`, `expense`, `tuition`, `buy_home`, `buy_to_let`, `investment`, `end`, `one_off_expense`, `one_off_income`, `death` — with their required fields
- **THEN** loading it yields a `Scenario` whose cashflow and asset lists contain the corresponding items, and whose event list contains the `end` and `death` events

### Requirement: Death event ends the simulation at its firing month
A `death` event SHALL be a supported event type with no body. When the event fires (at its `when` month), the system SHALL apply any other events scheduled for that month and settle the firing month's cashflows (including any scheduled `one_off_income` payouts), and SHALL then stop the simulation: no further months are simulated, no further events fire, and the run reports the firing month as the terminal month. A scenario that does not declare a `death` event SHALL run to the configured horizon exactly as before.

#### Scenario: Death event terminates the run
- **WHEN** a scenario declares `{ type: death, when: 60 }` and a horizon of 360 months
- **THEN** the simulator runs 61 simulated months (months 0..=60) and reports month 60 as the terminal month

#### Scenario: Death with no other events produces a single-month run
- **WHEN** a scenario declares only `{ type: death, when: 0 }`
- **THEN** the simulator runs exactly 1 simulated month (month 0) and reports it as the terminal month

#### Scenario: Scheduled payout fires at the death month
- **WHEN** a scenario declares `{ type: death, when: 24 }` and a `{ type: one_off_income, when: 24, one_off: { amount: 1000 } }`
- **THEN** month 24's `monthly_cashflow` includes the +1000 payout AND the run terminates after month 24
