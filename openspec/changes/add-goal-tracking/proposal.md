# Proposal: add-goal-tracking

## Why

ender tracks exactly one planning goal today — `saving: <floor>`,
which records whether cash ever dropped below a configured floor and
reports the first breach month. The `financial-plan` skill calls out
several other first-class planning goals: education funding ("can I
fund the kid's college by 2031?"), retirement corpus ("net worth
≥ $X at age 65"), estate target ("liquid net worth ≥ $Y at age 90").
Today, answering any of these requires reading the per-month array
manually and computing the answer outside ender.

A generic `goals:` list lets each scenario declare as many goals as
it has, and the run output reports one line per goal — `goal met` or
`goal missed (value at month N)` — without the user writing any
glue.

## What Changes

- Add an optional top-level `goals:` field to `ScenarioInput`, parsed
  as a list of `Goal { name: String, target: f64, by_month: u16,
  kind: GoalKind }` where `GoalKind` is `Cash` (default) or
  `NetWorth` (`cash + assets_value`).
- The simulator evaluates each goal at its `by_month` (or at the
  terminal month if the run is terminated earlier by a `death` event)
  and records whether the target was met:
  - `Cash`: `cash ≥ target` at `by_month`
  - `NetWorth`: `cash + assets_value ≥ target` at `by_month`
- The text summary appends one line per goal: `goal <name> met` or
  `goal <name> missed (<value> at month <N>)`, ordered by `by_month`.
- `--json` output adds a top-level `goals` field alongside the
  existing `terminal` / `months` fields: an array of
  `{name, target, by_month, kind, met: bool, value: f64}`.
- The existing `saving:` field continues to work unchanged. When a
  `goals:` list is also present, `saving:` is reported in addition
  to the per-goal lines. `saving:` remains the simplest floor; new
  scenarios SHOULD prefer `goals:` for any non-trivial planning
  question.
- Pure addition. Existing scenarios without `goals:` are unaffected.

## Capabilities

### New Capabilities

- `goal-tracking`: the simulator evaluates a list of cash /
  net-worth goals at their target months and reports met / missed for
  each in the run output.

### Modified Capabilities

- `scenario-loading`: add the optional `goals:` field to
  `ScenarioInput` alongside `saving`.
- `cli-output`: extend the text summary with per-goal lines and
  the `--json` wrapper with a `goals` field.

## Impact

- `src/main.rs`: a new `Goal` struct, a `GoalKind` enum, an optional
  `goals: Vec<Goal>` field on `ScenarioInput` and `Scenario`, a small
  `evaluate_goals(stats, goals) -> Vec<GoalOutcome>` helper, and the
  output-formatting updates in `format_text_summary` and `stats_json`.
- `key_stats` gains a `goals_outcomes: Vec<GoalOutcome>` field so the
  TUI can render the goal lines in its header.
- `skills/ender/SKILL.md`: document the `goals:` field with one
  worked example (college funding + retirement corpus).
- Tests: a `cash` goal met, a `cash` goal missed (with the value at
  the by_month reported), a `net_worth` goal met, a `net_worth` goal
  missed, multiple goals evaluated and reported in `by_month` order,
  `goals` absent produces no extra output, `saving:` and `goals:`
  coexist.