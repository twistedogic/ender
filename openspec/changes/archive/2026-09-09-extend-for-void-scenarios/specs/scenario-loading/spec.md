## ADDED Requirements

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