## Context

The only planning goal ender supports today is `saving: <floor>`,
declared at the scenario's top level. The field is checked on every
month: `Scenario::once` records the first month `cash < saving` and
`format_text_summary` appends `saving target breached at month N`
when a breach occurred.

The pattern works but is narrow:

- It can only express "cash stays above X" — not "net worth above X"
  (which is the natural retirement-corpus question) and not "cash
  reaches X by month N" (which is the education-funding question).
- It conflates two different questions: "do I ever run out?" (a floor)
  and "did I hit my target?" (a goal). A reader of the run output
  has to remember which is which.
- It is a single field, not a list, so a scenario that wants to
  express multiple goals (college + retirement + estate) must
  choose one or split into multiple runs.

The `financial-plan` skill lists these planning goals as distinct
questions (Step 4 Education Funding, Step 4 Estate Planning, Step 3
Distribution Phase). Generalizing `saving:` into a `goals:` list is
the smallest change that supports all three.

## Goals / Non-Goals

**Goals:**

- A `goals:` list on the scenario, each goal with `name`, `target`,
  `by_month`, and `kind` (`Cash` or `NetWorth`).
- The simulator evaluates each goal at its `by_month` (or at the
  terminal month when the run was terminated by a `death` event
  before `by_month`).
- The text summary reports one line per goal in `by_month` order:
  `goal <name> met` or
  `goal <name> missed (<value> at month <N>)`.
- `--json` output includes a top-level `goals` array with one entry
  per goal: `{name, target, by_month, kind, met, value}`.
- The existing `saving:` field continues to work. A scenario MAY
  declare both; both are reported.
- Tests for the goal evaluation, ordering, output, and `saving:`
  coexistence.

**Non-Goals:**

- Goal-based early termination. A missed goal is reported, not used
  to stop the run; the run continues to horizon (or `death`). A
  future change could add "stop at first missed goal" as a separate
  flag.
- Goal progress tracking (e.g., "70% of the way to the college
  fund at month 24"). The model records met/missed at `by_month`;
  intermediate progress is recoverable from the per-month array but
  not surfaced by the run.
- Goal dependencies (e.g., "fund the college goal from the
  retirement corpus only if the corpus exceeds $X"). The current
  model has no mechanism for cross-goal constraints; users compose
  scenarios by hand.
- Stochastic goal success (probability of meeting the goal). ender is
  deterministic; stress-test the goal with `downturn` events instead.
- Removing `saving:` — backward compat only; `saving:` continues to
  work and is documented as the simple floor variant.

## Decisions

### D1: New `goals:` list, `saving:` retained for backward compat
Add `goals: Vec<Goal>` to `ScenarioInput` alongside the existing
`saving: Option<f64>`. Both fields work; both are reported. The
`saving:` floor retains its semantics ("first month cash dropped
below") because that's a different question from goals.

*Alternative rejected*: replace `saving:` with a single-element
`goals` list — breaks existing scenarios. The two fields are
genuinely different (floor vs target-at-month) and users have
already invested in scenarios that use `saving:`.

### D2: Two kinds — `Cash` and `NetWorth`
`GoalKind` is `enum { Cash, NetWorth }` with `Cash` defaulting when
the YAML omits the field. `NetWorth` is `cash + assets_value` at
the by_month. This covers both the education-funding question (cash
in the kid's account by year X) and the retirement-corpus question
(net worth at retirement age).

*Alternative rejected*: a single `metric: cash|assets|net_worth`
field with three options — `assets` alone (without cash) is rarely
the right planning metric; collapsing to two options keeps the
schema honest. A future change could add `liquid_assets` (funds
minus mortgage face) if needed.

### D3: Goals evaluated at by_month or terminal month
The simulator looks up `stats[by_month]` (or the last stats
element if the run was terminated before `by_month`). For a
`death`-terminated run, goals past the terminal month are reported
as `met: false` with `value: <terminal cash + assets_value>` and
the terminal month — the user gets a clear "the goal was not
reached because the run ended at month N" signal.

*Alternative rejected*: evaluating `cash` and `assets_value`
across multiple months and reporting the closest-to-met trajectory
— more information but harder to read, and the planning question
is binary (did it happen by the target month or not?).

### D4: Goal lines in text summary, goals array in JSON
Text mode appends one line per goal in `by_month` order. JSON mode
adds a top-level `goals` array with `{name, target, by_month, kind,
met, value}` per entry. The TUI header adds one extra line per goal
when the run is displayed in TTY.

*Alternative rejected*: a goal summary in the TUI as a separate
pane — adds a third widget to the existing layout for a feature
that doesn't need one. The header is enough; users who want
per-month progress read the JSON.

## Risks / Trade-offs

- [Multiple goals at the same `by_month` produce multiple lines] →
  Acceptable: the order within a single `by_month` is the YAML
  order, which is the order the user wrote them.
- [Goals after the horizon are unreachable] → Rejected by the
  simulator; loading fails with an error if a goal's `by_month` is
  greater than the horizon. The user's intent ("fund college by
  month 600 in a 360-month run") is a scenario bug; surface it.
- [Goals past the terminal month on a `death`-terminated run are
  reported as missed with the terminal value] → Acceptable: the
  user can read the terminal month from the `terminal_month`
  field and understand the goal was unreached because of the
  earlier termination.

## Migration Plan

Pure addition. `ScenarioInput` gains one optional field, the
simulator gains a small evaluator function, the output gains a few
lines / one JSON array. Existing scenarios without `goals:` produce
byte-identical text output to the pre-change baseline (the new
formatting kicks in only when `goals:` is present). Rollback =
remove the field and the evaluator. No persisted state.

## Open Questions

None blocking. If a future change wants goal-based early
termination ("stop the run at the first missed goal"), that's a
separate capability. If a future change wants a third goal kind
("liquid assets" = funds minus mortgage face), it's a one-line
extension to `GoalKind`.