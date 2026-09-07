## MODIFIED Requirements

### Requirement: Scenario is defined in a YAML file
The system SHALL load a scenario from a YAML file containing starting `cash` and a list of `events`, where each event has a `when` (month index, 0-based) and a `type` discriminator. Supported event types SHALL include `job`, `expense`, `tuition`, `layoff`, `graduate`, `buy_home`, `buy_to_let`, `investment`, and `end`.

#### Scenario: Minimal scenario
- **WHEN** a YAML file contains `cash: 20000` and an empty `events` list
- **THEN** loading it yields a `Scenario` with 20000.0 cash, no cashflows, no assets, and no events

#### Scenario: Full event vocabulary
- **WHEN** a YAML file uses each event type — `job`, `expense`, `tuition`, `layoff`, `graduate`, `buy_home`, `buy_to_let`, `investment`, `end` — with their required fields
- **THEN** loading it yields a `Scenario` whose cashflow and asset lists contain the corresponding items, and whose event list contains the `end` event

## ADDED Requirements

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
