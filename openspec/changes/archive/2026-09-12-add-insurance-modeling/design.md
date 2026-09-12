## Context

ender's simulation model is a deterministic monthly clock: events
fire at their `when`, cashflows settle, the reserve (if any) is
restored, and the next month runs. The simulation ends at
`horizon` months (default 360), or earlier if the user sets
`end: "YYYY-MM"` to truncate the run.

The `financial-plan` skill treats insurance as a first-class
modeling primitive (Step 4 Risk Management) with three flavors:

1. **Life insurance** — a premium you pay while alive; a lump-sum
   payout to your estate / beneficiaries at death.
2. **Disability insurance** — a premium you pay while working; an
   income stream that replaces your salary if you become disabled.
3. **Long-term care insurance** — a premium you pay; a cost that
   becomes an outflow if you need care (often modeled as a large
   recurring expense in late retirement).

ender's existing primitives already cover the cashflow mechanics:

- Premium cost → `expense: {monthly, annualized_rate}` (recurring)
- Lump-sum payout → `one_off_income: {amount, when: ...}` (one-time)
- Income replacement → `end: {id: main-job}` + the cover modeled as
  a `pension:`-style inflow (see `add-pension-cashflow` for the
  analogous pattern).

The missing piece is the **terminal event** that ties these together.
Without a `death` event, modeling "die at 90 with a $1M policy"
forces the user to set `end: ...` to month 360 + (90 - retirement),
and even then the run continues to process the scheduled payout in
isolation — there's no shared "this is when death happens" marker.

A `death` event adds that marker and lets the run terminate cleanly
at the death month. Insurance is composed from the existing
primitives; this change just gives the user the terminal sentinel
and documents the pattern.

## Goals / Non-Goals

**Goals:**

- A `death` event with no body that ends the simulation at its
  firing month.
- Any `one_off_income` event scheduled at the same `when` as the
  death event SHALL fire in the death month (death and payout land
  together), so the terminal cash + assets reflect the post-payout
  state.
- The run output reports the terminal month, both in text mode and
  in the TUI header; `--json` mode adds a top-level `terminal`
  field that is `true` when the run was cut short by a death event
  and `false` otherwise.
- A "Risk management" section in `skills/ender/SKILL.md` with worked
  examples for life insurance and disability / income-protection.
- Two example scenarios under `scenarios/` exercising the pattern.

**Non-Goals:**

- New cashflow variant for insurance. Premium = expense, payout =
  one_off_income. Adding a dedicated `insurance_premium` variant
  buys nothing the existing primitives don't already give.
- Inheritance / estate distribution modeling. The `financial-plan`
  skill calls out "Estate Planning" (trusts, gifting, beneficiary
  review); those are policy / legal decisions outside ender's
  simulator scope. ender's role is to answer "does my estate have
  enough to cover X at death?" — the lump-sum `one_off_income`
  already does that.
- Long-term-care insurance as a separate event type. LTC is a
  recurring outflow (modeled as `expense`) that becomes active in
  late retirement; the pattern is identical to nursing-home cost
  modeling, which already works with today's primitives.
- "Death as a probability" — a stochastic death. ender is
  deterministic by design; the user picks the death month and the
  run answers "is the plan viable if death happens at this time?"
- A "what if death had not occurred" baseline run. That comparison
  is the responsibility of `add-multi-scenario-comparison` (a
  separate capability); this change provides the death event, not
  the comparison UI.

## Decisions

### D1: Death event with no body, terminal flag on Scenario
`EventType::Death {}` (no fields) sets a `terminated: bool` flag on
the `Scenario` struct. `Scenario::run` checks the flag at the top
of each iteration and returns the stats collected so far when set.
The flag is set in the `apply` arm of `Death`, so the death-month's
events fire and the cashflow settles before the next iteration
sees the flag.

*Alternative rejected*: an `at_month: u16` field on the scenario
top-level — clutters every scenario YAML for a feature most users
won't use. An event is the right shape: it composes with the
existing event list and is only present in scenarios that need it.

### D2: Death fires one_off_income at the same when
The death event and a scheduled `one_off_income` at the same `when`
both fire in the same month because `Scenario::once` processes all
firing-month events before the cashflow loop runs. The
`one_off_income`'s `fired` flag flips from false to true on the
firing iteration; the death's `terminated` flag flips at the same
moment; the run exits after the iteration completes. No new
ordering logic is needed.

*Alternative rejected*: a `trigger: death` field on `one_off_income`
to make the dependency explicit — couples the one-off to a separate
event type, makes "fire on death" a special case instead of an
ordinary `when`. Two ways to express the same timing is worse than
one.

### D3: JSON output gains a top-level `terminal` field, no per-month change
The existing `--json` shape is a flat array of per-month objects.
The minimal extension is to wrap that array in an object:
`{"terminal": false, "months": [...]}` when not terminated, or
`{"terminal": true, "terminal_month": N, "months": [...]}` when
terminated. The TUI header adds a single line; the text summary
adds a single line.

*Alternative rejected*: add a `terminal_month: N` field to the last
array element — mixes scenario metadata into per-month data and
forces consumers to special-case the last element. The wrapper is
cleaner.

### D4: Insurance is composed from existing primitives
The risk-management documentation in the skill walks through two
patterns:

- **Life insurance**: `expense: insurance_premium: {monthly, ...}`
  recurring cost from `when: 0`; `one_off_income: {amount:
  1000000, when: 1080}` (death at age 90 for a 30-year-old);
  `death: {when: 1080}` terminal marker. The terminal cash after
  the payout reflects "my estate has $X".
- **Disability / income protection**: `expense: cover_premium:
  {monthly, ...}` recurring cost; on the disability event, a
  `pension:` cashflow with the replacement amount starts (see
  `add-pension-cashflow`); `end: {id: main-job}` removes the
  salary. No death event needed — the run continues at a lower
  income level.

*Alternative rejected*: a dedicated `life_insurance: {premium,
payout, payout_on: death}` event — duplicates `one_off_income` +
`expense` and adds a new code path to maintain.

## Risks / Trade-offs

- [No stochastic death — user picks the month] → Acceptable: ender
  is deterministic, the user explores scenarios by varying the
  death month, and `add-multi-scenario-comparison` lets the user
  compare multiple death months side-by-side.
- [One_off_income must be at the same `when` as `death` for the
  payout to land before termination] → Documented as the pattern;
  the worked example in the skill pins the two `when` values
  together.
- [No estate-tax / inheritance-tax modeling] → Out of scope; HK
  has no estate tax. The skill notes this is an HK-specific gap
  not addressed by ender.
- [TUI table cuts off at the death month] → The table renders up
  to and including the terminal month; scrolling past is bounded
  by `stats.len()`. No special TUI handling needed beyond the
  existing row clamp.

## Migration Plan

Pure addition. `EventType` gains one variant, `Scenario` gains one
field, `Scenario::run` gains one early-exit branch, output gains
one line in two modes and a top-level field in `--json`. Existing
scenarios without `death` are unaffected — they run to `horizon`
exactly as today, with `terminal: false` in the JSON wrapper.
Rollback = remove the variant, the field, and the early-exit
branch. No persisted state.

## Open Questions

None blocking. If a future change wants `death: {at_age: 90}` with
age computed from a `birth_year` field on the scenario, that's a
separate capability. If a future change wants multiple death events
(modeling a spouse's death at a different month), the current
implementation supports them naturally (each `death` event sets the
flag, the run exits at the first one — second is a no-op).