## 1. Cashflow variant

- [ ] 1.1 Add `Pension { monthly: f64, annualized_rate: f64 }` to
  `CashflowItem` in `src/main.rs`, with the same field names as
  `Salary`.
- [ ] 1.2 Add a branch in `CashflowItem::monthly()`: read current
  `monthly`, multiply by `(1 + annualized_rate)^(1/12)`, return
  `(Cashflow::Income(m), 0.0)` — no MPF deduction.
- [ ] 1.3 Add a branch in `CashflowItem::tax_accrual()` returning
  `(monthly, 0.0)` — pension is added to `salary_accrued` (gross
  tax base) but NOT to `rent_accrued` or `mpf_accrued`.

## 2. Event variant

- [ ] 2.1 Add `Pension { pension: Pension }` to `EventType` where
  `Pension` is a small struct with `monthly` and `annualized_rate`.
- [ ] 2.2 Add an `apply` arm that pushes a `Labeled { id, item:
  CashflowItem::Pension(...) }` onto `Scenario::cashflow`, matching
  `EventType::Job`.

## 3. Tests

- [ ] 3.1 `pension_event_adds_cashflow`: scenario with one `pension`
  event has the cashflow in its list after `once()`.
- [ ] 3.2 `pension_is_income_no_mpf`: monthly_cashflow equals
  `+monthly` (no MPF deduction, no negative sign).
- [ ] 3.3 `pension_grows_like_salary`: two months of a pension
  match `Salary`'s month-2 monthly growth within float tolerance.
- [ ] 3.4 `pension_accrues_into_salaries_tax`: a scenario with
  only a pension of 20000 reports a salaries-tax charge at month 11
  computed against `salary_accrued = 240000` (zero MPF).
- [ ] 3.5 `pension_does_not_inflate_mpf_accrued`: a scenario with
  a salary and a pension reports `mpf_accrued` based on the salary
  only; the pension's presence changes neither the MPF total nor
  the cashflow in any month.
- [ ] 3.6 `pension_stacks_with_salary_in_tax_base`: salary 30000
  + pension 10000 for 12 months produces a salaries-tax charge
  computed against `salary_accrued = 480000`.
- [ ] 3.7 `end_removes_pension_by_id`: an `end` event with the
  pension's id removes it from the cashflow list.
- [ ] 3.8 `end_on_unknown_pension_id_is_noop`: existing
  `end_with_unknown_id_is_noop` test should continue to pass
  (covers pension too via the same code path).
- [ ] 3.9 `pension_without_body_fails_to_load`: missing
  `pension:` map yields an error naming the file.

## 4. Documentation

- [ ] 4.1 Add `pension` to the event-type table in
  `skills/ender/SKILL.md` with body `pension: {monthly,
  annualized_rate}` and effect "income, no MPF, assessable for
  salaries tax".
- [ ] 4.2 Add a worked example: retire at month 360 with
  `end: {id: main-job}`, add `pension` of 15k at month 360, observe
  that month-371 salaries tax reflects the pension base.

## 5. Validate

- [ ] 5.1 `cargo test` — all existing tests still pass, all new
  tests pass.
- [ ] 5.2 `cargo run --quiet -- scenarios/<retire-with-pension>.yaml`
  — run completes and the final cash matches the no-pension case
  except for MPF savings.
- [ ] 5.3 The existing scenario `scenario.yaml` (no pension)
  produces byte-identical output before and after this change —
  no regression.