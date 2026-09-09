## Context

`~/Dev/void/scenarios/` is 30+ scenario files (top-level + `rent3/`,
`layoff10/`, `layoff10-rehire/`, `lohas3/`, `noproperty/`, `retire55/`)
authored against an older, richer schema. The current ender schema is a
flat event list (`cash` + `events[]` with `when` as a month index) and
cannot express the void features that drive the scenarios' results:

- **Downturn** (equity + property drops) — used by every multi-decade
  scenario to model 1997/SARS/GFC/COVID-style crashes.
- **Refinance** (rate shock on an existing mortgage) — used by the buy
  scenarios to apply the 2022-style hiking cycle.
- **Household-configurable tax** (married/children/parents with age band
  and cohabitation) — drives the salaries-tax allowance for every void
  scenario with a salary.
- **Dated event triggers** (`start`, `end`) — used to anchor the 33-year
  horizon from 2026-08 to 2059-08.

Translating without these features would be lossy. We extend ender's
schema minimally to express them, then port the scenarios. The schema
extensions are the smallest changes that preserve scenario fidelity for
the four fields above. Stamp duty, home-loan-interest deduction,
domestic-rent deduction, and one-off tax rebates are out of scope
(see Non-Goals); their absence is documented in `scenarios/NOTES.md`.

## Goals / Non-Goals

**Goals:**

- A `downturn` event that drops fund principal and property
  `price_per_sqft` in a single month, with independent knobs per asset
  class.
- A `refinance` event that replaces a mortgage's monthly payment by id.
- A `tax:` block that overrides the basic-allowance code constant with a
  household-derived total (status + children + parents), using 2026/27
  IRD figures.
- Optional `start`/`end`/`saving` fields that let the run truncate to a
  real date horizon and track a cash floor.
- Translated `scenarios/*.yaml` for every void source file, loadable by
  the extended ender.
- Existing scenarios (`scenario.yaml`, tests) load unchanged.

**Non-Goals:**

- Stamp duty on property purchases (HK AVD Scale 2 is a piecewise table;
  the translated scenarios note this gap but don't model it).
- Home-loan-interest deduction in salaries tax (cap HKD 120,000/yr).
- Domestic-rent deduction in salaries tax (cap HKD 120,000/yr).
- One-off government tax reductions (e.g., 2025/26 rebate).
- Calendar alignment (Apr–Mar, paid in arrears) — the sim's month-index
  clock stays; `start: "2026-08"` is informational anchor, not a calendar.
- Mortgage model rework — mortgages remain `monthly + period + paid`; a
  refinance precomputes the new `monthly` at translation time and the
  event simply replaces it. Outstanding-principal tracking is not added.
- TUI changes — the run output formats are unchanged; the new fields
  appear only when configured.

## Decisions

### D1: Downturn mutates asset values in place, no cash flow
`Downturn` lives on `EventType` alongside `Job`, `Layoff`, etc. It
iterates `Scenario::assets` once: each `Fund::principal *= (1 −
equity_drop)`; each `Property::price_per_sqft *= (1 − property_drop)`.
Capex is left alone (it's not a market value; it's a one-time sunk cost
in the model). No cash flow, no per-month smoothing — the drop lands in
the firing month and monthly growth resumes from the post-drop base on
month +1.

*Alternative rejected*: a per-month `growth_multiplier` field — more
state, same answer, more tests.

### D2: Refinance replaces `monthly` on the matched mortgage
`Refinance { id, monthly }` walks `Scenario::assets`, matches the first
`Property::mortgage.id` (and would match `Fund::investment` if we ever
supported refinanceable fund contributions), and replaces the inner
`CashflowItem::Mortgage::monthly`. The replacement takes effect on the
firing month's cashflow iteration; we keep the existing per-month
mutation pattern (growth applied at the top of `Asset::monthly()`).

*Alternative rejected*: switching the mortgage to
`principal + rate + period` and recomputing monthly on the fly —
realistic, but doubles the mortgage code surface and breaks the existing
test suite. Refinance in ender is a translation-time convenience; the
new monthly is computed against the original principal and remaining
period when the void scenario is ported.

### D3: Tax block → derived total allowance, single override point
A new `TaxConfig` struct deserialized from the optional `tax:` block.
`TaxConfig::annual_basic()` returns the sum: `basic(status) +
children × 140_000 + Σ parent_allowance(age_band, living_with)`. The
result is computed once at scenario load and stored on `Scenario` as
`allowance_override: Option<f64>`. `Scenario::once()` reads the override
when calling `salaries_tax`; when absent, the existing `BASIC_ALLOWANCE`
constant is used, preserving today's behavior.

Constants live in the same module as the bracket table, alongside a
comment naming the IRD year of assessment. Adding a future year is a
one-constant edit.

*Alternative rejected*: threading `TaxConfig` through every tax call —
more types for the same single lookup. The override is loaded once.

### D4: `start`/`end`/`saving` as flat fields on `ScenarioInput`
All three fields deserialize onto `ScenarioInput` directly. `start` and
`end` parse `"YYYY-MM"` via a tiny helper (`fn ym(s: &str) -> (i32,
u32)`), producing `(year, month)` pairs. The horizon in months is
`months_between(end, start) + 1` (inclusive of the end month); the run
calls `run(horizon)` instead of the fixed `run(360)`. `saving` flows
through to `Scenario` as `Option<f64>`; `Scenario::once()` tracks the
minimum cash and the first breach. The existing `format_text_summary`
gains a `saving target breached at month N` line when applicable.

*Alternative rejected*: parsing dates into a calendar library — sim
still has no calendar, the date is informational.

### D5: Scenarios translate at write-time, not via a runtime shim
Each void source file becomes a hand-translated YAML under `scenarios/`,
mirroring the source directory layout. A mechanical conversion script
(small Python or shell) does the date-to-month math and the
mortgage-monthly precomputation; the result is reviewed before commit.
The translated file carries a `# ponytail:` header listing the dropped
features (stamp duty, rent/mortgage deductions, etc.) for that scenario.

*Alternative rejected*: a runtime adapter that reads the old schema and
emits the new one — more code, more drift risk, harder to reason about
than committed YAML.

## Risks / Trade-offs

- [Translated scenarios miss stamp duty → lower down-payment impact]
  → Documented in `scenarios/NOTES.md`; acceptable because every scenario
  flags its conclusions with conservative assumptions anyway. Add duty
  later as a config knob if precision matters.
- [Translated scenarios miss rent/mortgage-interest tax deductions →
  higher tax bill, earlier insolvency] → Documented; the conservative
  bias is consistent with void's own modeling notes.
- [Mortgage refinance uses precomputed monthly, not live recompute] →
  Void's scenarios all precompute at translation time anyway, so the
  result matches. If a future scenario amortizes a single loan
  dynamically, this design would need D2 revisited.
- [Tax block uses 2026/27 IRD figures; IRD may revise them] → Single
  edit point in the constants; spec already mentions the year. Add a
  test pinning current numbers.
- [Run output grows new fields only when configured] → Backward
  compatible. The `saving target breached` line only appears when
  `saving` is set.

## Migration Plan

Pure addition. All four new event types and three new top-level fields
are optional; existing scenarios (`scenario.yaml`, every test fixture)
load and run identically. Rollback = drop the new variants from
`EventType` and the new fields from `ScenarioInput`; no persisted state
to migrate. Translated scenarios live under a new `scenarios/`
directory and are referenced by name only — nothing points at them by
default.

## Open Questions

None blocking. Future work (parked): stamp duty as a config knob, live
mortgage amortization, full rent/mortgage-interest tax deductions,
calendar alignment. None are needed for the current translation.