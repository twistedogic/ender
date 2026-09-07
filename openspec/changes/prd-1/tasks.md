## 1. One-off expense (TODO 1)

- [ ] 1.1 Write a failing test mirroring spec scenario "One-off expense lands once": `one_off_expense` with `when: 3`, `one_off: { amount: 5000 }`, cash 20000 — month 3 `monthly_cashflow` is −5000, cash 15000, later months' flow 0. Confirm the failure mode is the unknown event type.
- [ ] 1.2 Implement `CashflowItem::OneOff { amount, fired }` (signed amount, `#[serde(default)] fired`), the `monthly()` arm (`fired` → zero, else set flag and return `Cashflow::new(amount)`), and `EventType::OneOffExpense(Labeled)` storing `-amount.abs()`, pushed in `apply()` like the other adders. Test goes green.
- [ ] 1.3 Failing test for spec scenario "One-off without fired flag": a YAML one-off specifying only `amount` loads and fires exactly once.

## 2. One-off income (TODO 2)

- [ ] 2.1 Write a failing test mirroring "One-off income lands once": `one_off_income` with `when: 6`, `one_off: { amount: 10000 }` — month 6 flow +10000, later months 0.
- [ ] 2.2 Implement `EventType::OneOffIncome(Labeled)` storing `amount.abs()`, reusing the item variant. Test goes green.

## 3. Edges and integration

- [ ] 3.1 Failing test mirroring "Sign is taken from the event type": `one_off_expense` with `amount: -5000` behaves as +5000.
- [ ] 3.2 Failing test mirroring "One-off may carry an id": an `end` event with the one-off's id, firing before the one-off's month, removes it; the one-off contributes nothing.
- [ ] 3.3 Update the full-vocabulary load test to include both one-off events; all existing tests stay green (`cargo test`); run the CLI on `scenario.yaml` and confirm identical output to before the change.

## 4. Bookkeeping and verification

- [ ] 4.1 Check off TODOS items 1 and 2.
- [ ] 4.2 `openspec validate prd-1 --strict` passes.
