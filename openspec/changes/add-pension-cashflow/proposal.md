# Proposal: add-pension-cashflow

## Why

ender has `salary` (recurring income, less MPF) and `rental_income`
(recurring income, scaled by occupancy). Pension income — a recurring
post-retirement inflow, **not** subject to MPF, often indexed to
inflation, and treated as assessable income under Hong Kong salaries
tax — has no first-class representation. The only way to model a
pension today is to lie about it being a salary (`salary: {monthly: 0}`)
and accept that MPF is silently being deducted and the wrong tax base
is being computed, which corrupts both the cashflow signal and the
annual salaries-tax charge.

The `financial-plan` skill calls this out explicitly: Step 1 lists
"pensions" alongside salary and Social Security as a primary income
source; Step 3 Distribution Phase treats pension income as the
post-retirement counterpart to salary. Adding `pension` as a separate
cashflow variant gets this right without forcing the user to abuse
`salary`.

## What Changes

- Add `CashflowItem::Pension { monthly: f64, annualized_rate: f64 }`,
  structurally identical to `Salary` minus MPF and plus an inflation
  indexing knob. The variant's `monthly` is the gross pension amount
  (no employee-side MPF deduction), and `annualized_rate` is the
  indexing rate (HK civil-service pensions are typically CPI-linked).
- Add `EventType::Pension { id: Option<String>, pension: Pension }` so
  a scenario can introduce a pension at a chosen month (e.g.
  `when: 360`).
- Pension income feeds `salary_accrued` for the annual salaries-tax
  computation, the same way `salary` does. This matches HK practice:
  pension is assessable income subject to the same progressive /
  standard-rate regime.
- Pension does NOT deduct MPF, so the `mpf_accrued` total is unchanged
  by pension income.
- Pension does NOT contribute to `rent_accrued` for property tax.
- Pure addition: existing scenarios load and run identically.

## Capabilities

### New Capabilities

- `pension-income`: the simulator can model recurring pension income
  as a non-MPF cashflow that is assessable under salaries tax.

### Modified Capabilities

- `scenario-loading`: extend the `events[]` vocabulary with `pension`.
- `hk-salaries-tax`: include pension gross in `salary_accrued` so the
  annual tax charge is computed against total retirement income
  alongside any residual earned income.

## Impact

- `src/main.rs`: one new `CashflowItem` variant (`Pension`), one new
  `EventType` variant (`Pension`), a `monthly()` branch that returns
  `Cashflow::Income(m)` (no MPF), and a `tax_accrual()` branch that
  returns `(monthly, 0.0)` so the pension is added to `salary_accrued`
  but not `mpf_accrued` or `rent_accrued`.
- `EventType::apply` gains a `Pension` arm pushing a labeled
  `CashflowItem::Pension` onto `Scenario::cashflow`, mirroring
  `EventType::Job`.
- `skills/ender/SKILL.md`: document `pension` in the event-type
  table; add a note that pension income is assessable for salaries
  tax but not subject to MPF.
- Tests: pension is positive cashflow, no MPF, taxable; pension +
  salary accrue together; pension is removable via `end: {id}`; an
  existing scenario with only `job` events produces the same
  salaries-tax bill it does today (zero-impact regression test).