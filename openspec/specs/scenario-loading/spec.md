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

### Requirement: Scenario declares a goals list
The scenario file SHALL accept an optional top-level `goals:` field alongside `cash`, `events`, and `saving`. The field SHALL be a list of goals; absent or empty SHALL be equivalent to no goals declared and SHALL NOT change the run's text or TUI rendering relative to a scenario without the field. Each goal SHALL carry four fields: `name` (a non-empty string), `target` (a positive number), `by_month` (a 0-based month index, SHALL be ≤ the scenario's horizon — otherwise loading fails with an error naming the file), and `kind` (`cash` or `net_worth`; default `cash`). Two goals SHALL NOT share both `name` and `by_month` (duplicate goal identity); duplicates SHALL fail to load with an error naming the file. Goals are evaluated post-run by capability `goal-tracking`.

#### Scenario: Goals list parses
- **WHEN** a scenario declares two goals with `kind: cash` and `kind: net_worth`
- **THEN** loading succeeds and the scenario carries two goals with the stated names, targets, by_months, and kinds

#### Scenario: Goal past horizon fails to load
- **WHEN** a goal's `by_month` exceeds the scenario's horizon
- **THEN** loading fails with an error naming the file and identifying the offending goal

### Requirement: Scenario may declare a cash reserve
The scenario file SHALL accept an optional top-level `reserve_months` field: a non-negative integer number of months of recurring expenses to hold as cash. When the field is absent or `0`, no reserve requirement SHALL apply and behavior SHALL be identical to a scenario without the field. A `reserve_months` value that is not a non-negative integer SHALL fail loading with the existing file error reporting.

#### Scenario: Reserve loads
- **WHEN** a YAML file specifies `reserve_months: 3` alongside `cash` and `events`
- **THEN** loading succeeds and the scenario carries a 3-month reserve requirement

#### Scenario: Absent reserve disables settlement
- **WHEN** a scenario with no `reserve_months` field runs a month whose flows take cash negative while a fund asset exists
- **THEN** nothing is liquidated and cash simply goes negative, exactly as before this change

#### Scenario: Negative reserve fails to load
- **WHEN** a YAML file specifies `reserve_months: -1`
- **THEN** loading fails with an error naming the file

### Requirement: Settlement restores the cash reserve by liquidation in order
When a reserve is configured, the system SHALL, after applying each month's flows to cash, compute the reserve target as `reserve_months ×` that month's recurring expense total — the sum of all negative flows of the month excluding one-off items' firing amounts, including embedded asset flows such as mortgage payments and fund contributions. If cash is at or above the target, nothing SHALL happen. Otherwise the shortfall SHALL be funded in a fixed order: first by drawing fund principal (partially, in asset order, each fund floored at zero), then by selling whole properties in asset order. A property's sale proceeds SHALL be its current market value minus its mortgage's remaining total payments (`monthly × (period − paid)` as of that month), and the sale SHALL remove the asset so its mortgage and rental flows stop in later months. Liquidation SHALL stop as soon as cash reaches the target; sale overshoot SHALL remain in cash. Liquidation proceeds and drawdowns SHALL NOT appear in `monthly_cashflow`. The reserve SHALL be breached — cash left below the target, possibly negative — only when all funds and properties are exhausted, and a month that ends with cash below zero after settlement SHALL be reported by the run as insolvent.

#### Scenario: Fund drawn to top up the reserve
- **WHEN** a scenario with `reserve_months: 3`, cash 100, a rent of 200 per month, and a fund with principal 1000 runs its first month
- **THEN** the month's flow of −200 leaves cash at −100, settlement draws 700 from the fund, cash ends at the 600 target, the fund's principal ends at 300, and `monthly_cashflow` is −200

#### Scenario: One-off does not inflate the reserve target
- **WHEN** a scenario with `reserve_months: 3`, cash 8000, a rent of 2000 per month, a fund with principal 10000, and a `one_off_expense` of 5000 firing in month 0 runs that month
- **THEN** the reserve target is 6000 (rent only), settlement draws 5000 — cash ends at exactly 6000 — not the 20000 shortfall a one-off-inflated target would force

#### Scenario: Property sold after funds are exhausted
- **WHEN** a scenario with `reserve_months: 3`, cash 100, a rent of 200 per month, a fund with principal 100, and a property worth 30000 with a mortgage of 100 per month over 12 periods runs its first month
- **THEN** the fund is drawn to zero, the property is then sold for 28900 (30000 value minus the 1100 remaining mortgage payments), cash ends at 28800, the asset list is empty, and the next month's `monthly_cashflow` is −200 — the rent only, the sold property's mortgage flow gone with it

#### Scenario: Reserve breached only when assets are exhausted
- **WHEN** a scenario with `reserve_months: 3`, cash 50, a rent of 200 per month, a fund with principal 100, and no properties runs its first month
- **THEN** the fund is drawn to zero, cash ends at −50 below the 600 target with no assets left, and the run reports that month as insolvent

#### Scenario: Cash above target is untouched
- **WHEN** a scenario with `reserve_months: 3`, cash 50000, and a rent of 2000 per month runs a month whose net flow keeps cash above 6000
- **THEN** no liquidation occurs and assets are unchanged

### Requirement: Downturn event drops funds and property values in one month
A `downturn` event SHALL apply a one-shot percentage drop to every fund's
`principal` and to every property's `price_per_sqft` in the month the
event fires. The event SHALL carry two fields: `equity_drop` (applied to
funds, expressed as a positive fraction in `[0, 1)`) and `property_drop`
(applied to properties, same convention). Each drop SHALL be applied as
`new_value = value × (1 − drop)`. The event SHALL NOT affect capex, the
mortgage balance, or cash directly; subsequent monthly growth resumes at
the configured `annualized_rate` from the post-drop base. A scenario that
declares only one of the two drops SHALL leave the other asset class
untouched. Downturn SHALL fire exactly once at its `when` and SHALL NOT
appear in `monthly_cashflow`.

#### Scenario: Full drop halves both asset classes
- **WHEN** a scenario holds a fund with principal 1,000,000 and a property
  with `price_per_sqft: 30,000` and a downturn event with
  `equity_drop: 0.5, property_drop: 0.5` fires
- **THEN** the fund's principal becomes 500,000, the property's
  `price_per_sqft` becomes 15,000, `monthly_cashflow` is unchanged, and
  the assets resume growing at their configured rates from those bases

#### Scenario: Only equity drop leaves properties intact
- **WHEN** a downturn event with `equity_drop: 0.22, property_drop: 0.0`
  fires against a fund and a property
- **THEN** the fund loses 22% of its principal and the property's
  `price_per_sqft` is unchanged

#### Scenario: Downturn does not refire
- **WHEN** a downturn event with `when: 36` is loaded and the scenario runs
  for 60 months
- **THEN** the drops are applied once at month 36 and never again

### Requirement: Refinance event replaces a mortgage's monthly payment
A `refinance` event SHALL replace the `monthly` value of a mortgage
identified by `id` at the month the event fires. The event SHALL carry an
`id` (the `Labeled::id` on the target mortgage's enclosing property asset)
and a `monthly` (the new payment amount, non-negative). The replacement
SHALL leave `period`, `paid`, and the property's other fields untouched.
The event SHALL be a silent no-op when no asset's mortgage carries the
matching id. Refinance SHALL NOT appear in `monthly_cashflow`; the new
payment takes effect on the next month's cashflow iteration.

#### Scenario: Refinance updates the monthly on the matching property
- **WHEN** a property asset has `mortgage.id: home-loan` with
  `monthly: 30000, period: 300`, and a refinance event with
  `id: home-loan, monthly: 42000` fires
- **THEN** the property's mortgage `monthly` becomes 42000 in the firing
  month and subsequent months; `monthly_cashflow` for the firing month is
  unchanged, and the next month reflects the higher payment

#### Scenario: Unknown refinance id is a no-op
- **WHEN** a refinance event with `id: ghost` fires and no asset carries
  that id
- **THEN** no mortgage is modified, the scenario continues, and no error
  is reported

#### Scenario: Refinance without an id fails to load
- **WHEN** a refinance event is declared without an `id`
- **THEN** loading fails with an error naming the file

### Requirement: Scenario declares a start date, end date, and saving target
The scenario file SHALL accept three optional top-level fields alongside
`cash` and `events`: `start` (a `"YYYY-MM"` string anchoring month 0 of
the run), `end` (a `"YYYY-MM"` string limiting the run horizon in months
from `start`, inclusive), and `saving` (a non-negative cash floor the run
tracks). When `start` is absent, month 0 is the implied anchor and the
field is informational only. When `end` is present, the simulator SHALL
stop after `months(end) − months(start) + 1` simulated months; when
absent, the run uses the default 360-month horizon. When `saving` is
present, the run SHALL record whether `cash` ever dropped below it and the
first month it did; when absent, no target is tracked. All three fields
SHALL be optional; absence is identical to today's behavior.

#### Scenario: End truncates the run
- **WHEN** a scenario declares `start: 2026-08, end: 2029-08`
- **THEN** the simulator runs 37 months (months 0..=36) and the reported
  `final_monthly_cashflow` corresponds to month 36

#### Scenario: Saving target is tracked
- **WHEN** a scenario declares `saving: 100000` and a month ends with cash
  at 80,000
- **THEN** the run records that the saving target was breached at that
  month and reports it in the summary

#### Scenario: Absent end uses default 360-month horizon
- **WHEN** a scenario declares no `end` field
- **THEN** the run executes 361 simulated months (months 0..=360), as
  before

#### Scenario: Negative saving fails to load
- **WHEN** a scenario declares `saving: -1`
- **THEN** loading fails with an error naming the file

