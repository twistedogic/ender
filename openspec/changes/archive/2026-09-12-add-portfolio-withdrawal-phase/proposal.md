# Proposal: add-portfolio-withdrawal-phase

## Why

ender simulates accumulation (salary → funds → property) but has no concept
of decumulation: post-retirement, monthly spending is funded from assets,
not income. The `financial-plan` skill calls this out as Step 3
"Distribution Phase" (sustainable withdrawal rate, "Portfolio at 90"),
which ender currently cannot answer. Today the only way to model
retirement spending is to hand-author a large `one_off_expense` per month
and accept that no drawdown happens — funds sit there compounding while
cash goes negative, which is the opposite of what retirement looks like.

The existing reserve machinery (`reserve_months` → `settle()`) already
implements the correct asset drawdown order: fund principals first, then
whole-property sales in asset order. This change is the missing trigger:
a "retired at month N, spend X/yr, let the reserve mechanism fund the
shortfall" cashflow.

## What Changes

- Add `CashflowItem::Withdrawal { monthly: f64, annualized_rate: f64 }`,
  structurally identical to `Rent` and `Tuition` (monthly amount grows by
  `^(1/12)`, contributes to `expense_base`, never positive).
- Add `EventType::Withdrawal { withdrawal: Withdrawal }` so a scenario can
  introduce a withdrawal cashflow at a specific month (e.g. `when: 360`).
- `Withdrawal` is taxable under salaries tax the same way any other
  inflow is — but in practice pension / annuity income fills that gap (see
  `add-pension-cashflow`); the model stays agnostic here.
- No new liquidation mechanics. The existing `reserve_months` /
  `settle()` / fund-then-property order is the answer; users set
  `reserve_months` to whatever buffer they want funded from assets.
- Pure addition: existing scenarios load and run identically.

## Capabilities

### New Capabilities

- `decumulation`: the simulator can model a recurring drawdown cashflow
  that grows with inflation, starts at a chosen month, and is funded from
  the existing reserve / asset liquidation pipeline when cash is short.

### Modified Capabilities

- `scenario-loading`: extend the `events[]` vocabulary with `withdrawal`.

## Impact

- `src/main.rs`: one new `CashflowItem` variant, one new `EventType`
  variant, one branch in `monthly()` matching `Rent`/`Tuition`, and
  one branch in `tax_accrual()` returning zero (withdrawal is not a
  salary or rent). `Withdrawal` count toward `expense_base` like `Rent`
  does, so the existing reserve formula picks them up automatically.
- `skills/ender/SKILL.md`: document the new event in the event table and
  add a worked example ("retire at month 360, spend 30k, 6-month
  reserve").
- Tests: `withdrawal` adds cashflow with growth, contributes to
  reserve base, and is indistinguishable from `Rent` for the simulator's
  purposes (no MPF, no tax, no asset side-effect).