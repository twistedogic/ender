# Proposal: extend-for-void-scenarios

## Why

The `~/Dev/void/scenarios/` family (30+ files across `rent3/`, `layoff10/`,
`layoff10-rehire/`, `lohas3/`, `noproperty/`, `retire55/`, plus top-level)
was authored against an older, richer schema with `downturn` (equity +
property drops), `refinance` (rate shock on an existing mortgage), a
configurable `tax:` block (married, children, parent allowances), and dated
event triggers (`at: 2029-08`). The current ender schema has none of these
features, so converting the scenarios faithfully is impossible. We need to
extend the schema enough that translation is lossless for the core
fidelity-bearing fields (downturn, refinance, household-configurable tax,
date anchoring), then port the 30+ scenarios under `scenarios/`.

## What Changes

- Add a `downturn` event: applies a one-shot percentage drop to fund
  principal and to property `price_per_sqft` in a single month. Two knobs:
  `equity_drop` (funds) and `property_drop` (properties).
- Add a `refinance` event: replaces a mortgage's `monthly` payment, identified
  by `id`. Used to model the 2022-style rate shock void scenarios apply to
  existing mortgages.
- Extend the `tax:` block with household configuration: `status: single |
  married`, `children: u8`, `parents: [{age_band, living_with}]`. The total
  allowance is derived from this and replaces the hardcoded `BASIC_ALLOWANCE`
  for salaries-tax computation. Uses 2026/27 IRD figures (basic 145k single,
  290k married; child 140k; parent 60+ 55k / 110k living-with; 55–59 27.5k /
  55k living-with).
- Add scenario-level `start: "YYYY-MM"`, `end: "YYYY-MM"`, `saving: f64`
  fields. `start` is informational (anchor for dated event translation);
  `end` is the run horizon in months from `start` (default 360); `saving`
  is a post-run target — the run reports whether cash ever dropped below
  it. Used by void's `saving: 1400000` floor.
- Translate all `~/Dev/void/scenarios/**/*.yaml` files under `scenarios/`
  using a mechanical pass that maps dated `at:` events to month indices
  from `start`, precomputes monthly mortgage payments from the
  `down/loan/rate/months` tuples void uses, and drops features the engine
  still doesn't model (stamp duty, home-loan-interest deduction, domestic
  rent deduction, one-off tax rebates). Documented in `scenarios/NOTES.md`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `scenario-loading`: add `downturn` and `refinance` events; add top-level
  `start` / `end` / `saving` fields with the semantics above.
- `hk-salaries-tax`: when a `tax:` block is present, derive total allowance
  from `status` / `children` / `parents` and pass it through to
  `salaries_tax`, overriding the default constant. Add an `annual_basic`
  helper that sums the parts.

## Impact

- `src/main.rs`: two new `EventType` variants (`Downturn`, `Refinance`),
  three new optional fields on `ScenarioInput` (`start: String`,
  `end: String`, `saving: f64`), `TaxBlock` deserialization + allowance
  derivation. Mortgage id targeting for refinance reuses the existing
  `Labeled` id plumbing.
- `scenario.yaml` schema: new optional top-level fields + expanded `tax:`
  block. All existing scenarios continue to load unchanged.
- Tests: downturn (equity + property drops), refinance (mortgage by id),
  tax block (allowance computation for each combination of
  married/children/parents), start/end horizon length, saving target
  reporting.
- New `scenarios/` directory with translated files (one per void source
  file, preserving the directory structure). Void's `notes.md` is *not*
  copied; an ender-native `scenarios/NOTES.md` documents the gaps.