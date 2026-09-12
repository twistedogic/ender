# Proposal: add-insurance-modeling

## Why

The `financial-plan` skill devotes a whole section (Step 4 Risk
Management) to life, disability, and long-term-care insurance. ender
already supports most of the cashflow plumbing for these products via
existing primitives:

- Premium = `expense: {monthly, annualized_rate}` (recurring cost)
- Lump-sum payout = `one_off_income: {amount}` (one-time inflow)
- Income replacement = `end: {id: main-job}` (stop work) plus
  `expense: {monthly: 0}` for the cover that replaces it

What's missing is the **terminal event** that makes "die at 90, payout
$1M" a real scenario instead of "die at 90, the run keeps going
pretending the dead still pay rent". Today the only way to end a
scenario at a chosen month is to set `end: "YYYY-MM"` to that month,
which is informative but doesn't carry the semantic weight of "this
is the death date — payout fires here, everything stops".

A `death` event fixes the terminal scenario without adding new
cashflow machinery, and lets the skill document insurance patterns
with worked examples that the run actually exercises end-to-end.

## What Changes

- Add `EventType::Death {}` with no body. When the event fires, the
  simulation SHALL stop after settling the firing month: no further
  months are simulated, no further events fire, and the run output
  reports the firing month as the terminal month.
- Insurance policies are modeled with the existing primitives:
  premium as a recurring `expense`, payout as a scheduled
  `one_off_income` whose `when` is set to the death month. The `death`
  event acts as a sentinel that ends the run; the scheduled payout
  fires in the same month and lands in cash before the run reports.
- No new cashflow variant, no new asset kind, no new reserve regime.
  The simulator already has every primitive the insurance pattern
  needs; this proposal wires them together with a terminal-event
  marker and documents the pattern.
- Pure addition. Existing scenarios without a `death` event are
  unaffected — they run to `horizon` (or `end`) exactly as today.

## Capabilities

### New Capabilities

- `terminal-scenarios`: the simulator supports a `death` event that
  ends the simulation at a chosen month, after the firing month's
  cashflow and any scheduled one-offs have settled.

### Modified Capabilities

- `scenario-loading`: extend the `events[]` vocabulary with `death`.

## Impact

- `src/main.rs`: one new `EventType::Death` variant with no body; an
  `apply` arm that flips a `terminated: bool` flag on `Scenario`; and
  an early-exit in `Scenario::run` that returns the stats collected so
  far once `terminated` is set. The terminal month is the last element
  of the returned stats slice.
- `Scenario::once` continues to apply the firing month's events and
  flows before the death-handling check at the end of `run`. This
  ensures any `one_off_income` event scheduled at the same `when` as
  the `death` event fires in that terminal month (death and payout
  land together).
- `format_text_summary` and `key_stats` add a single new line when
  the run was terminated by a death event: "terminal at month N
  (death event)". The TUI header gains the same line. In `--json`
  mode, a top-level `terminal` field is added (true when the run was
  terminated early, false otherwise) alongside the existing per-month
  array — matching the existing `--json` shape but with one extra
  field, which downstream consumers parse forward-compatibly
  (extra fields are ignored).
- `skills/ender/SKILL.md`: add `death` to the event-type table; add
  a "Risk management / insurance modeling" section with two worked
  examples (life insurance, income protection / disability).
- Tests: `death` event terminates the run, scheduled `one_off_income`
  at the same `when` fires, terminal line appears, `death` before any
  other event produces a single-month run, scenario without `death`
  runs to horizon unchanged.