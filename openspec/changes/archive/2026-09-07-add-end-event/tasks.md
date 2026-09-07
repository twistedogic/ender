## 1. Spike: serde id plumbing

- [x] 1.1 Write a failing test that loads `{ type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.0 } }` and asserts the cashflow item carries id `partner`. Confirm the failure mode (missing `id` field vs flatten error).
- [x] 1.2 If plain `#[serde(default)] id: Option<String>` + `#[serde(flatten)]` of `CashflowItem` inside the internally-tagged `EventType` does not deserialize, implement the fallback (nested YAML shape or small custom `Deserialize` for adder events) and record which path was taken in design.md.

## 2. Labeled envelope refactor (no behavior change)

- [x] 2.1 Introduce `Labeled { id: Option<String>, item: CashflowItem }`; change `Scenario.cashflow` to `Vec<Labeled>`; delegate `monthly()` through the envelope; make `layoff`/`graduate` removals match on `labeled.item` variant. All existing tests stay green.
- [x] 2.2 Change asset-embedded item fields to labeled: `Fund.investment: Option<Labeled>`, `Property.rental: Option<Labeled>`, `Property.mortgage: Labeled`; update `Asset::monthly` and existing tests accordingly.

## 3. End event

- [x] 3.1 Failing test: two labeled salaries (`me`, `partner`), `end` with `id: partner` fires — only partner's salary stops (mirror of spec scenario "End removes a specific salary").
- [x] 3.2 Failing test: `end` on a labeled fund contribution stops the monthly contribution while principal keeps compounding (mirror of spec scenario "End stops a fund contribution").
- [x] 3.3 Failing test: `end` with an id no active item carries changes nothing and reports no error.
- [x] 3.4 Implement `EventType::End { id: String }` with the scan from design decision 3: top-level cashflow list, then assets in order (rental, mortgage; fund investment), remove first match.

## 4. Verification

- [x] 4.1 Update the full-vocabulary load test to include `end` and ids; run `cargo test`; run the CLI on `scenario.yaml` (no ids) to confirm identical output to before the change.
- [x] 4.2 `openspec validate add-end-event --strict` passes.
