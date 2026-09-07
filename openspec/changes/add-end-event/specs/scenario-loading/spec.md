## MODIFIED Requirements

### Requirement: Scenario is defined in a YAML file
The system SHALL load a scenario from a YAML file containing starting `cash` and a list of `events`, where each event has a `when` (month index, 0-based) and a `type` discriminator. Supported event types SHALL include `job`, `expense`, `layoff`, `graduate`, `buy_home`, `buy_to_let`, `investment`, and `end`.

#### Scenario: Minimal scenario
- **WHEN** a YAML file contains `cash: 20000` and an empty `events` list
- **THEN** loading it yields a `Scenario` with 20000.0 cash, no cashflows, no assets, and no events

#### Scenario: Full event vocabulary
- **WHEN** a YAML file uses each event type — `job`, `expense`, `layoff`, `buy_home`, `buy_to_let`, `investment`, `end` — with their required fields
- **THEN** loading it yields a `Scenario` whose cashflow and asset lists contain the corresponding items, and whose event list contains the `end` event

## ADDED Requirements

### Requirement: Cashflow items may carry an id label
Every cashflow item, wherever it appears — added by a top-level event (`job`, `expense`, `inition`) or embedded in an asset (fund `investment`, property `rental`, property `mortgage`) — SHALL accept an optional string `id`. Items without an `id` SHALL load and behave exactly as before.

#### Scenario: Labeled salary loads
- **WHEN** a `job` event specifies `{ type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.03 } }`
- **THEN** loading succeeds and the resulting cashflow item carries id `partner`

#### Scenario: Unlabeled scenario is unchanged
- **WHEN** a scenario file with no `id` fields anywhere is loaded
- **THEN** loading succeeds and behavior is identical to before this change

### Requirement: End event removes the labeled item matching its id
An `end` event SHALL remove the first cashflow item whose `id` equals the event's `id`, scanning the top-level cashflow list first, then assets in order. If no item carries a matching id, the event SHALL be a silent no-op. Duplicate ids SHALL resolve to the first match in scan order.

#### Scenario: End removes a specific salary
- **WHEN** two `job` events labeled `me` and `partner` are active, and an `end` event with `id: partner` fires
- **THEN** only the `partner` salary stops; the `me` salary continues in later months

#### Scenario: End stops a fund contribution
- **WHEN** an `investment` event created a fund whose `investment` contribution is labeled `401k`, and an `end` event with `id: 401k` fires
- **THEN** the fund's principal keeps growing at its rate but no further monthly contribution is added or subtracted from cash

#### Scenario: Unknown id is a no-op
- **WHEN** an `end` event fires with an id no active item carries
- **THEN** nothing is removed, the simulation continues, and no error is reported
