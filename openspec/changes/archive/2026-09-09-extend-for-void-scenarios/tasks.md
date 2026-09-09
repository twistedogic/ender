## 1. Schema: tax block + allowance override

- [x] 1.1 Add `TaxConfig` struct (`status`, `children`, `parents:
  Vec<ParentConfig>`) with serde deserialization; reject unknown fields
  via `#[serde(deny_unknown_fields)]`.
- [x] 1.2 Add `annual_basic(&self) -> f64` returning the household-derived
  total per the spec (145k / 290k basic + 140k/child + 55k or 110k parent
  60+ / 27.5k or 55k parent 55–59, doubled when `living_with: true`).
- [x] 1.3 Tests for `annual_basic`: married+2 kids+2 non-cohabiting parents
  60+ → 680,000; single+1 cohabiting parent 55–59 → 200,000; empty block
  → returns the default constant unchanged.

## 2. Schema: start / end / saving fields

- [x] 2.1 Add `start: Option<String>`, `end: Option<String>`, `saving:
  Option<f64>` to `ScenarioInput`; thread them into `Scenario` as
  `Option<(i32, u32)>`, `Option<(i32, u32)>`, `Option<f64>`.
- [x] 2.2 Add a tiny `fn ym(s: &str) -> Result<(i32, u32), String>`
  parser; reject malformed strings with a clear error.
- [x] 2.3 Compute the horizon from `end − start + 1` months when `end` is
  present; default to 360 when absent. Plumb into `main()` so `run(horizon)`
  is used.
- [x] 2.4 Track minimum cash and the first month `cash < saving` in
  `Scenario::once()`; render the breach in `format_text_summary` when
  applicable.
- [x] 2.5 Tests: end truncates the run; saving breach reports a month;
  absent end uses the 360 default; negative saving fails to load; bad
  `"YYYY-MM"` string fails to load.

## 3. Event: downturn

- [x] 3.1 Add `EventType::Downturn { equity_drop: f64, property_drop: f64 }`.
- [x] 3.2 Implement `Scenario::apply_downturn(...)` that mutates each
  asset in place per the spec (`principal *= (1 − equity_drop)`,
  `price_per_sqft *= (1 − property_drop)`); no cashflow entry.
- [x] 3.3 Tests: full drop halves both; only equity drops property is
  untouched; downturn does not refire across later months.

## 4. Event: refinance

- [x] 4.1 Add `EventType::Refinance { id: String, monthly: f64 }`.
- [x] 4.2 Implement `Scenario::apply_refinance(...)` that finds the first
  property asset whose `mortgage.id == id` and replaces
  `CashflowItem::Mortgage::monthly`. No-op when no match.
- [x] 4.3 Tests: refinance updates the matching mortgage's monthly and
  leaves period/paid alone; unknown id is a no-op; missing `id` fails to
  load.

## 5. Wire taxes to the override

- [x] 5.1 Compute the override at scenario-load time and store on
  `Scenario` as `allowance_override: Option<f64>`.
- [x] 5.2 In `Scenario::once()`, when calling `salaries_tax(...)`, pass
  `allowance_override.unwrap_or(BASIC_ALLOWANCE)`.
- [x] 5.3 Test: married+2 kids scenario pays tax against the derived
  total, not the code constant.

## 6. Translate void scenarios

- [x] 6.1 Write a mechanical converter (small Python script under
  `scripts/translate_void.py`) that walks `~/Dev/void/scenarios/**/*.yaml`,
  computes month indices from `start: 2026-08`, precomputes mortgage
  monthly from `(loan, rate, months)`, and emits ender-format YAML under
  `scenarios/` mirroring the source tree.
- [x] 6.2 Run the converter; review each emitted file for sanity (cash +
  events present, dates line up, mortgage monthly matches a hand
  calculation).
- [x] 6.3 Add a `# ponytail:` header to each file listing dropped features
  (stamp duty, rent deduction, mortgage-interest deduction) and the
  saving target when relevant.
- [x] 6.4 Add `scenarios/NOTES.md` summarizing the dropped features and
  any scenario-specific annotations.

## 7. Validate

- [x] 7.1 Run `cargo test` — all existing tests still pass, all new tests
  pass.
- [x] 7.2 For each translated scenario, run the binary against it and
  confirm the run completes without panicking; the JSON output is
  well-formed and the saving target is reported when breached.
- [x] 7.3 Hand-check one scenario (e.g., `scenarios/rent3/01-rent-forever.yaml`)
  against the void output for the same parameters (cash trajectory shape,
  min cash month, insolvent month).