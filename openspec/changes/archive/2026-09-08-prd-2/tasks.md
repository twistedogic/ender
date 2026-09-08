## 1. Reserve field (TODO 3, loading side)

- [x] 1.1 Write a failing test mirroring spec scenario "Reserve loads": `reserve_months: 3` loads and the `Scenario` carries it. Confirm the failure mode is the missing field.
- [x] 1.2 Write a failing test mirroring "Negative reserve fails to load": `reserve_months: -1` fails with an error naming the file.
- [x] 1.3 Add `#[serde(default)] reserve_months: u32` to `ScenarioInput`, carry it into `Scenario` in `into_scenario()`. Tests go green.

## 2. Settlement (TODO 3 + 4, simulation side)

- [x] 2.1 Write a failing test mirroring "Cash above target is untouched": reserve 3, cash 50000, rent 2000 — no liquidation, assets unchanged. Confirm it fails only after `settle()` exists (red against the new code path, not the field).
- [x] 2.2 Write a failing test mirroring "Fund drawn to top up the reserve": reserve 3, cash 100, rent 200, fund 1000 — cash ends 600, principal 300, `monthly_cashflow` −200.
- [x] 2.3 Restructure `once()` to compute flows and the recurring expense base in one pass (each `monthly()` called exactly once; `OneOff` items' values count toward the flow, never the base), then after `cash += cashflow` call a new `settle()` that no-ops when `reserve_months == 0` or cash ≥ target, else draws fund principals partially in asset order. Tests 2.1–2.2 go green.
- [x] 2.4 Write a failing test mirroring "One-off does not inflate the reserve target": reserve 3, cash 8000, rent 2000, fund 10000, `one_off_expense` 5000 at month 0 — cash ends exactly 6000 (draw 5000, not 15000+).
- [x] 2.5 Write a failing test mirroring "Property sold after funds are exhausted": fund drawn to zero, property sold for value − remaining mortgage face (28900), cash 28800, asset list empty, next month's flow −200 (rent only, the sold property's mortgage gone).
- [x] 2.6 Extend `settle()` with whole-property sales in asset order: proceeds = `sqft × price_per_sqft − monthly × (period − paid)` (post-payment state), asset removed, stop when cash reaches target or assets run out. Test 2.5 goes green.
- [x] 2.7 Write a failing test mirroring "Reserve breached only when assets are exhausted": fund drawn to zero, cash −50, run reports that month insolvent.
- [x] 2.8 Write a failing test mirroring "Absent reserve disables settlement": no `reserve_months`, month takes cash negative with a fund present — nothing liquidates (today's behavior pinned).
- [x] 2.9 In `main`, scan the stats for the first month with cash below zero and print `insolvent from month N` after the summary line when one exists.

## 3. Bookkeeping and verification

- [x] 3.1 Update the full-vocabulary load test to include `reserve_months`; all existing tests stay green (`cargo test`); run the CLI on `scenario.yaml` with `reserve_months: 3` added and confirm the summary line is unchanged and no insolvent line prints.
- [x] 3.2 Check off TODOS items 3 and 4 (all four intent items now shipped).
- [x] 3.3 `openspec validate prd-2 --strict` passes.
