## ADDED Requirements

### Requirement: Scenario declares a pension event that adds a non-MPF recurring income cashflow

The scenario file SHALL accept a new event type `pension` alongside
`job`, `expense`, `tuition`, and the other existing variants. The
event SHALL carry a body map `pension: {monthly, annualized_rate}`,
mirroring the shape of `salary`. When the event fires, the system
SHALL add a `Pension` cashflow item to the scenario's cashflow
list whose `monthly` and `annualized_rate` equal the body's fields.
The cashflow's `monthly` SHALL grow by
`(1 + annualized_rate)^(1/12)` per simulated month, identical to
how `salary`/`rent`/`tuition` grow.

The `monthly` amount SHALL be paid in full into the scenario's cash
each month — no MPF deduction SHALL apply to pension income. The
pension's gross monthly SHALL be added to the `salary_accrued`
total that drives the annual salaries-tax charge, the same way
`salary` is. The pension SHALL NOT add to `mpf_accrued` or
`rent_accrued`.

#### Scenario: Pension event adds a non-MPF income cashflow

- **WHEN** a scenario declares `{ type: pension, when: 360,
  pension: {monthly: 15000, annualized_rate: 0.025} }`
- **THEN** after month 360 the cashflow list contains a `Pension`
  item with `monthly: 15000.0` and `annualized_rate: 0.025`, and
  the next simulated month reports a `monthly_cashflow` of
  `+15000.0` (not `+15000 - MPF`) and `cash` increases by 15000
  from the prior month.

#### Scenario: Pension is not subject to MPF

- **WHEN** a pension of 40000 is active
- **THEN** the scenario's `mpf_accrued` total at the next
  month-11 boundary is unchanged by the pension's presence; an
  identical scenario without the pension reports the same
  `mpf_accrued`. A pension of 40000 that fires for 12 months
  contributes exactly `12 * 40000 = 480000` to `salary_accrued`,
  matching how a 40000 salary would contribute if there were no MPF.

### Requirement: Pension income is assessable under salaries tax

Pension gross SHALL be added to `salary_accrued` for the purposes
of the annual salaries-tax charge, alongside any `salary` items.
The tax charge SHALL be computed using the same progressive vs
standard-rate comparison as for earned income, subject to the
scenario's `tax:` allowances block.

#### Scenario: Pension accrues into salaries tax with no salary

- **WHEN** a scenario has only a pension of 20000 active for a
  full 12 months and no salary items
- **THEN** at month 11 the salaries-tax charge is computed against
  a `salary_accrued` of `12 * 20000 = 240000` minus MPF (zero)
  minus the scenario's allowance, exactly as if the pension were a
  salary of 20000 with no MPF — which it is, by HK salaries-tax law.

#### Scenario: Pension stacks with salary in the salaries-tax base

- **WHEN** a scenario has a salary of 30000 and a pension of 10000
  active for a full 12 months
- **THEN** `salary_accrued` is `12 * (30000 + 10000) = 480000` and
  `mpf_accrued` is `12 * mpf_monthly(30000)` — pension does not
  contribute to MPF but does contribute to the salaries-tax base.

### Requirement: Pension is removed by an end event with matching id

The `Pension` cashflow SHALL be removable by an `end` event whose
`id` matches the pension's `id`, identical to how `end` removes a
salary. Without a matching id, `end` is a silent no-op for pension.

#### Scenario: End removes a labeled pension

- **WHEN** a scenario declares a pension event with `id: civ_serv`
  and a later event `{ type: end, when: 480, id: civ_serv }`
- **THEN** the `Pension` item is removed from the cashflow list at
  month 480 and the months from 481 onwards carry no pension
  outflow/inflow.

#### Scenario: End on unknown pension id is a no-op

- **WHEN** an `end` event fires with `id: ghost` and no pension
  carries that id
- **THEN** nothing is removed, the simulation continues, and no
  error is reported — matching the existing `end`-no-op behavior
  on salary / rent / tuition.

### Requirement: Pension event without body fails to load

A `pension` event without a `pension:` body, or with a body that
is missing `monthly` or `annualized_rate`, SHALL fail to load
with an error naming the file, matching the validation pattern for
`job` / `expense` / `tuition`.

#### Scenario: Pension event rejected without body

- **WHEN** a scenario declares `type: pension` with no `pension`
  body
- **THEN** loading fails with an error naming the file.