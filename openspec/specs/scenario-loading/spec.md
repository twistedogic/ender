# scenario-loading Specification

## Purpose

Load a scenario (starting cash + events) from a YAML file into a runnable `Scenario`, with defined error reporting for invalid files.
## Requirements
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

### Requirement: Cashflow items may carry an id label
Every cashflow item, wherever it appears — added by a top-level event (`job`, `expense`, `tuition`, `one_off_expense`, `one_off_income`) or embedded in an asset (fund `investment`, property `rental`, property `mortgage`) — SHALL accept an optional string `id`. Items without an `id` SHALL load and behave exactly as before.

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

### Requirement: Asset acquisition deducts its price from cash
When a `buy_home` or `buy_to_let` event fires, the system SHALL deduct `sqft × price_per_sqft` from cash in that month. When an `investment` event fires, the system SHALL deduct `principal` from cash in that month. Each deduction SHALL occur exactly once, in the month the event fires. These one-time deductions SHALL NOT appear in the reported monthly cashflow; mortgage payments and monthly contributions SHALL continue unchanged.

#### Scenario: Home purchase deducts full price
- **WHEN** a `buy_home` event with `sqft: 1200` and `price_per_sqft: 300` fires with cash at 50000
- **THEN** cash for that month reflects 50000 − 360000, the property enters the asset list, and the month's monthly cashflow contains no purchase amount

#### Scenario: Fund purchase deducts principal
- **WHEN** an `investment` event with `fund: { principal: 5000 }` fires with cash at 20000
- **THEN** cash for that month reflects 20000 − 5000, and the fund's principal still starts at 5000 and compounds from there

#### Scenario: Deduction happens once
- **WHEN** a scenario with a `buy_home` event runs for months after the purchase month
- **THEN** later months' cash changes only by their monthly cashflows; the purchase price is never deducted again

### Requirement: Layoff and graduate remove a targeted item
The `layoff` and `graduate` events SHALL accept an optional `id`. With `id`, the event SHALL remove the first cashflow item carrying that id from the top-level cashflow list; if no item carries it, the event SHALL be a silent no-op. Without `id`, the event SHALL remove the first item of its kind — the first salary for `layoff`, the first tuition for `graduate` — preserving existing behavior.

#### Scenario: Layoff with id removes the right salary
- **WHEN** two `job` events labeled `me` and `partner` are active, and a `layoff` event with `id: partner` fires
- **THEN** only the `partner` salary stops; the `me` salary continues in later months

#### Scenario: Layoff without id removes first salary
- **WHEN** a scenario with unlabeled salaries fires a `layoff` event with no `id`
- **THEN** the first salary in the cashflow list is removed, as before this change

#### Scenario: Graduate with id removes the labeled tuition
- **WHEN** a tuition item labeled `kid-2` is active alongside another tuition, and a `graduate` event with `id: kid-2` fires
- **THEN** only the `kid-2` tuition stops

#### Scenario: Layoff with unknown id is a no-op
- **WHEN** a `layoff` event fires with an id no active item carries
- **THEN** nothing is removed, the simulation continues, and no error is reported

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

