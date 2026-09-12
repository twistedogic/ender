## 1. Schema

- [ ] 1.1 Add `Goal { name: String, target: f64, by_month: u16, kind:
  GoalKind }` and `GoalKind { Cash, NetWorth }` to `src/main.rs`,
  with `#[serde(rename_all = "snake_case", default)]` for the kind
  field.
- [ ] 1.2 Add `goals: Vec<Goal>` to `ScenarioInput` with
  `#[serde(default)]`.
- [ ] 1.3 Validation at load time: reject duplicate goals (same
  `name` + `by_month`); reject goals whose `by_month` exceeds the
  scenario horizon; reject empty `name` and non-positive `target`.
  Errors SHALL name the file.

## 2. Evaluator

- [ ] 2.1 Add a `GoalOutcome { name, target, by_month, kind, met,
  value }` struct.
- [ ] 2.2 Add `fn evaluate_goals(stats: &[Stats], goals: &[Goal],
  terminal_month: Option<u16>) -> Vec<GoalOutcome>` that orders
  outcomes by `by_month` (then by input order), looks up
  `stats[min(by_month, terminal or horizon)]` for each goal, and
  computes `value` per kind.
- [ ] 2.3 Plumb `goals: Vec<Goal>` and `goals_outcomes: Vec<GoalOutcome>`
  through `Scenario` and `Scenario::once` / `Scenario::run` — the
  evaluation is a post-run computation, not per-month.

## 3. Output (text + TUI)

- [ ] 3.1 Update `format_text_summary` to append one line per goal
  in the format described in the spec.
- [ ] 3.2 Update `key_stats` to carry `goals_outcomes`.
- [ ] 3.3 Update `tui::run` / `tui::ui` to render the goal lines
  in the header paragraph; grow the header `Constraint::Length`
  from 7 by `goals.len()` rows.

## 4. Output (JSON)

- [ ] 4.1 Update `stats_json` to emit the wrapper object with the
  new `goals` field (empty array when no goals). Each entry uses
  the keys `name`, `target`, `by_month`, `kind` (`"cash"` /
  `"net_worth"`), `met`, `value`. Update the function signature
  to accept `goals_outcomes: &[GoalOutcome]`.

## 5. Tests

- [ ] 5.1 `goal_cash_met`: cash ≥ target at by_month reports met
  with the correct value.
- [ ] 5.2 `goal_cash_missed`: cash < target at by_month reports
  missed with value and target.
- [ ] 5.3 `goal_net_worth_uses_assets`: net_worth kind adds
  assets_value to cash.
- [ ] 5.4 `goal_default_kind_is_cash`: omitting kind yields cash.
- [ ] 5.5 `goals_evaluated_in_by_month_order`: two goals,
  different by_months, output is in ascending by_month.
- [ ] 5.6 `goal_past_horizon_fails_to_load`: by_month > horizon
  yields a load error naming the file.
- [ ] 5.7 `duplicate_goal_fails_to_load`: same name + same
  by_month twice yields a load error.
- [ ] 5.8 `empty_goals_produces_no_extra_output`: byte-identical
  text summary to a scenario without the field.
- [ ] 5.9 `saving_and_goals_coexist`: both fields reported,
  neither suppresses the other.
- [ ] 5.10 `json_goals_array_always_present`: empty array when
  no goals, populated array when goals present, correct types per
  element.

## 6. Skill + scenarios

- [ ] 6.1 Add `goals:` to the scenario schema documentation in
  `skills/ender/SKILL.md` with one worked example (college +
  retirement corpus).
- [ ] 6.2 Create `scenarios/goals-college.yaml` exercising the
  cash-kind goal.
- [ ] 6.3 Create `scenarios/goals-retirement.yaml` exercising the
  net-worth kind goal.
- [ ] 6.4 Create `scenarios/goals-saved.yaml` with both `saving:`
  and `goals:` to exercise the coexistence path.

## 7. Validate

- [ ] 7.1 `cargo test` — all existing tests still pass, all new
  tests pass.
- [ ] 7.2 Run each new scenario end-to-end: text summary includes
  the goal line(s), `--json` output parses with a populated
  `goals` array.
- [ ] 7.3 Run `scenario.yaml` (no `goals:`) and confirm
  byte-identical text output to the pre-change baseline.
- [ ] 7.4 Run `cargo run --quiet -- --json scenario.yaml` and
  confirm the wrapper's `goals` field is `[]` and the rest of
  the shape is unchanged.