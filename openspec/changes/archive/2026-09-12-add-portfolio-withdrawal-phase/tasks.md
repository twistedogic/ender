## 1. Cashflow variant

- [ ] 1.1 Add `Withdrawal { monthly: f64, annualized_rate: f64 }` to
  `CashflowItem` in `src/main.rs`, mirroring the fields of `Rent` and
  `Tuition`.
- [ ] 1.2 Add a branch in `CashflowItem::monthly()` matching `Rent`
  exactly: read current `monthly`, multiply by
  `(1 + annualized_rate)^(1/12)`, return
  `(Cashflow::Expense(m), m.abs())`.
- [ ] 1.3 Add a branch in `CashflowItem::tax_accrual()` returning
  `(0.0, 0.0)` (withdrawal is not a salary or rent).

## 2. Event variant

- [ ] 2.1 Add `Withdrawal { withdrawal: Withdrawal }` to `EventType`
  where `Withdrawal` is a small struct with the same fields as
  `Rent`/`Tuition`'s inner type.
- [ ] 2.2 Add an `apply` arm that pushes a `Labeled { id, item:
  CashflowItem::Withdrawal(...) }` onto `Scenario::cashflow` (matching
  `EventType::Expense`).

## 3. Serde plumbing

- [ ] 3.1 Confirm `serde_yaml_ng` derives pick up the new variant
  automatically (the existing variants use `#[derive(Deserialize)]`
  without field tags). Add a `deny_unknown_fields`-style assertion
  test mirroring `inition_is_rejected` for a `withdrawal` event
  missing its `withdrawal` body.

## 4. Tests

- [ ] 4.1 `withdrawal_event_adds_cashflow`: scenario with one
  `withdrawal` event has the cashflow in its list after `once()`.
- [ ] 4.2 `withdrawal_grows_like_rent`: two months of a withdrawal
  match `Rent`'s month-2 monthly growth within float tolerance.
- [ ] 4.3 `withdrawal_is_net_no_mpf`: monthly_cashflow equals
  `-monthly` (no MPF deduction, no salaries-tax accrual observable
  in cashflow; the cashflow signal is just the outflow).
- [ ] 4.4 `end_removes_withdrawal_by_id`: an `end` event with the
  withdrawal's id removes it from the cashflow list.
- [ ] 4.5 `withdrawal_in_reserve_base`: a scenario with
  `reserve_months: 6`, a rent of 10000, and a withdrawal of 30000
  settles to the `(6 * 40000)` target, not `(6 * 10000)`.
- [ ] 4.6 `withdrawal_triggers_asset_drawdown`: a scenario with only
  a fund and a withdrawal draws from the fund to maintain the
  reserve target month after month.
- [ ] 4.7 `withdrawal_without_body_fails_to_load`: missing
  `withdrawal:` map yields an error naming the file.

## 5. Documentation

- [ ] 5.1 Add `withdrawal` to the event-type table in
  `skills/ender/SKILL.md` with the same one-line description style
  as `expense`.
- [ ] 5.2 Add a worked retirement example to the skill: stop salary
  + rent at month 360, add a `withdrawal` of 30k with 2.5%
  inflation, set `reserve_months: 12`, run for 600 months.
- [ ] 5.3 Add a small `scenarios/retire-withdrawal.yaml` exercising
  the example so it is runnable.

## 6. Validate

- [ ] 6.1 `cargo test` — all existing tests still pass, all new
  tests pass.
- [ ] 6.2 `cargo run --quiet -- scenarios/retire-withdrawal.yaml` —
  run completes, text summary includes the final cash and assets
  at the chosen horizon.
- [ ] 6.3 `cargo run --quiet -- --json scenarios/retire-withdrawal.yaml
  | jq '.[-1]'` — final element is well-formed and matches the
  text summary's final-month values.