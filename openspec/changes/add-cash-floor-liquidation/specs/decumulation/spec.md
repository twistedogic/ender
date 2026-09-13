## ADDED Requirements

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

- **WHEN** a scenario declares `cash: 50`, `reserve_months: 0`, a rent
  expense of `200/mo`, and a fund asset with `principal: 1000` and
  `annualized_rate: 0.0`
- **THEN** after one simulated month, the scenario's `cash ≥ 0.0` and
  the fund's `principal` is reduced by the amount needed to cover the
  post-rent shortfall (was `−1150` with the fund untouched; now drawn
  from the fund up to its full 1000 principal).

#### Scenario: Positive cash with no reserve does not trigger settlement

- **WHEN** a scenario declares `cash: 50000`, `reserve_months: 0`, and a
  rent expense of `2000/mo` (post-rent cash = `48000`)
- **THEN** after one simulated month, `settle()` does not run (cash is
  above the zero floor), the cash balance remains `48000`, and no
  liquidation has occurred.

#### Scenario: Cash-zero with positive reserve restores to buffer target

- **WHEN** a scenario declares `cash: 100`, `reserve_months: 3`, a rent
  expense of `200/mo`, and a fund asset with `principal: 1000`
- **THEN** after one simulated month, the simulator reports `cash` at
  or above `3 × 200 = 600` (the buffer target), with the fund drawn to
  cover the shortfall up to that target.