# decumulation Specification

## Purpose

Model a post-retirement recurring drawdown cashflow that starts at a chosen
month and is funded from the existing reserve / asset liquidation pipeline
when cash is short. The reserve machinery already implements the right drawdown
order (funds first, then properties); this capability is the trigger.
## Requirements
### Requirement: Scenario declares a withdrawal event that starts a recurring drawdown cashflow

The scenario file SHALL accept a new event type `withdrawal` alongside
`job`, `expense`, and the other existing variants. The event SHALL
carry a single body map `withdrawal: {monthly, annualized_rate}`,
mirroring the shape of `rent` and `tuition`. When the event fires,
the system SHALL add a `Withdrawal` cashflow item to the scenario's
cashflow list whose `monthly` and `annualized_rate` equal the body's
fields, and SHALL treat that item exactly like `Rent` thereafter:
monthly grows by `(1 + annualized_rate)^(1/12)` per simulated month,
contributes its `monthly` to the reserve's `expense_base`, and
contributes nothing to MPF or salaries tax / property tax accruals.
The event SHALL NOT appear in any month's `monthly_cashflow` (it
registers a cashflow, like `job` and `expense`, rather than a flow).

#### Scenario: Withdrawal event adds a recurring expense cashflow

- **WHEN** a scenario declares `{ type: withdrawal, when: 360,
  withdrawal: {monthly: 30000, annualized_rate: 0.025} }`
- **THEN** after month 360 the cashflow list contains a `Withdrawal`
  item with `monthly: 30000.0` and `annualized_rate: 0.025`, and the
  next simulated month reports a `monthly_cashflow` of `-30000.0`
  and a `cash` reduced by 30000 from the prior month (no MPF, no
  tax accrual).

#### Scenario: Withdrawal grows with inflation like Rent

- **WHEN** a withdrawal of 30000 with `annualized_rate: 0.025` is
  active and the simulator advances one month
- **THEN** the cashflow's `monthly` becomes `30000 *
  (1.025)^(1/12)` and the month's `monthly_cashflow` reflects that
  larger amount, identical to how `Rent` of the same parameters
  behaves.

#### Scenario: Withdrawal is removed by an end event with matching id

- **WHEN** a withdrawal event is added with `id: spend` and a later
  event `{ type: end, when: 480, id: spend }` fires
- **THEN** the `Withdrawal` item is removed from the cashflow list
  and the months from 480 onwards no longer carry its outflow, the
  same way `end` removes a salary or rent.

#### Scenario: Withdrawal is rejected without required fields

- **WHEN** a scenario declares `type: withdrawal` with no `withdrawal`
  body, or with `withdrawal: {monthly: 30000}` (no `annualized_rate`)
- **THEN** loading fails with an error naming the file, matching the
  pattern used for other malformed events.

### Requirement: Withdrawal contributes to the reserve base

When a `Withdrawal` cashflow is active, its `monthly` SHALL be
included in the per-month `expense_base` used to compute the
`reserve_months` target, the same way `Rent` and `Tuition` are. As a
result, a larger withdrawal raises the reserve target proportionally,
and the existing `settle()` machinery (draw fund principals, then
sell whole properties) covers any cash shortfall caused by the
drawdown.

#### Scenario: Reserve target includes withdrawal

- **WHEN** a scenario declares `reserve_months: 6`, a rent of 10000,
  and a withdrawal of 30000 that is active
- **THEN** the reserve target is `6 * (10000 + 30000) = 240000`, not
  `6 * 10000 = 60000`, and `settle()` draws from assets to maintain
  cash at or above 240000 whenever a month ends below.

#### Scenario: Withdrawal alone funds reserve via asset drawdown

- **WHEN** a scenario declares `reserve_months: 12`, cash 0, a fund
  with `principal: 1000000` and `annualized_rate: 0.0`, and a
  withdrawal of 30000 starting at month 0 with `annualized_rate: 0.0`
- **THEN** month 0's flow is `-30000`, the reserve target is
  `12 * 30000 = 360000`, the shortfall is `360000`, and `settle()`
  draws 360000 from the fund leaving `principal: 640000` and cash
  at 360000; subsequent months continue drawing against the fund
  until it is exhausted.

### Requirement: Cash-zero trigger fires settle regardless of reserve_months

The `settle()` function SHALL fire whenever the post-event cash balance
is less than or equal to zero, regardless of the scenario's
`reserve_months` value. The restore target remains
`reserve_months × expense_base`; when `reserve_months = 0` this target
evaluates to `0.0`, and `settle()` draws just enough from funds (then
sells whole properties in asset order) to bring cash back to
non-negative. This decouples the "don't go cash-negative" safety net
from the "keep N months of expenses in cash" buffer, which is the role
of `reserve_months` when set to a positive value.

#### Scenario: Cash-zero with no reserve triggers fund drawdown

- **WHEN** a scenario declares `cash: 200`, `reserve_months: 0`, a rent
  expense of `200/mo`, and a fund asset with `principal: 1000` and
  `annualized_rate: 0.0`
- **THEN** after one simulated month, the scenario's `cash = 0.0` and
  the fund is fully drained (`principal = 0`, asset list empty).
  Without this behavior, the same scenario would leave `cash = -1000`
  with the fund untouched.

#### Scenario: Positive cash with no reserve does not trigger settlement

- **WHEN** a scenario declares `cash: 50000`, `reserve_months: 0`, and a
  rent expense of `2000/mo` (post-rent cash = `48000`)
- **THEN** after one simulated month, `settle()` does not run (cash is
  above the zero floor), the cash balance remains `48000`, and no
  liquidation has occurred.

#### Scenario: Cash-zero with positive reserve restores to buffer target

- **WHEN** a scenario declares `cash: 800`, `reserve_months: 3`, a rent
  expense of `200/mo`, and a fund asset with `principal: 1000` and
  `annualized_rate: 0.0`
- **THEN** after one simulated month, the scenario's `cash = 600`
  (the buffer target `3 × 200`), with the fund fully drained to cover
  the shortfall up to that target.
