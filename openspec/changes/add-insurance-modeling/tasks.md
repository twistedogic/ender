## 1. Event variant + Scenario field

- [ ] 1.1 Add `EventType::Death {}` with no body to
  `src/main.rs`. Add an `apply` arm that sets a new
  `terminated: bool` field on `Scenario` to `true`.
- [ ] 1.2 Add `terminated: bool` to the `Scenario` struct
  (default `false`) and seed it in `ScenarioInput::into_scenario`.

## 2. Termination logic

- [ ] 2.1 In `Scenario::run`, at the top of each iteration
  (before `once`), check `self.terminated`; if set, return the
  stats collected so far without further `once` calls.
- [ ] 2.2 Confirm `Scenario::once` continues to apply firing-
  month events in their existing order (death applies alongside
  `one_off_income` at the same `when`); no special ordering
  needed.

## 3. Output (text + TUI)

- [ ] 3.1 In `format_text_summary`, append a `terminal at month
  N (death event)` line when `scenario.terminated` is true; omit
  otherwise. Existing lines stay byte-identical.
- [ ] 3.2 In `tui::run`, pass the `terminated` flag through to
  the header; render one extra line `terminal: month N (death
  event)` when set. Adjust the `Paragraph` height constraint
  from 7 to 8 to accommodate the new line.

## 4. Output (JSON)

- [ ] 4.1 Refactor `stats_json` to wrap the existing per-month
  array in an object: `{terminal: bool, months: [...]}` and,
  when `terminal` is true, `terminal_month: usize`. Update the
  function signature to accept the termination flag.
- [ ] 4.2 Confirm the per-month array shape (each element with
  `month`, `cash`, `assets_value`, `monthly_cashflow`) is
  unchanged inside `months`.

## 5. Tests

- [ ] 5.1 `death_event_terminates_run`: scenario with
  `{type: death, when: 60}` returns 61 stats and the last is
  month 60.
- [ ] 5.2 `death_with_no_other_events_produces_one_month`:
  scenario with only `death: when: 0` returns 1 stat for month 0.
- [ ] 5.3 `scheduled_payout_fires_at_death_month`:
  `death: when: 24` + `one_off_income: when: 24, amount: 1000`
  reports month 24 cashflow of `+1000` and a terminal flag set.
- [ ] 5.4 `no_death_event_runs_to_horizon`: existing scenarios
  run to horizon unchanged, `terminal` flag is false.
- [ ] 5.5 `text_summary_reports_termination`: scenario with
  `death: when: 12` produces text output ending with the
  `terminal at month 12 (death event)` line.
- [ ] 5.6 `json_output_wraps_array`: `--json` output parses as
  `{terminal: true, terminal_month: 24, months: [...]}` for a
  terminated run and `{terminal: false, months: [...]}` for a
  non-terminated run.
- [ ] 5.7 `json_per_month_shape_unchanged`: every element of
  `months` in the new format has the same keys as the old
  format (`month`, `cash`, `assets_value`, `monthly_cashflow`).

## 6. Skill + scenarios

- [ ] 6.1 Add `death` to the event-type table in
  `skills/ender/SKILL.md` with the one-line description
  "ends the simulation at the firing month; pair with scheduled
  `one_off_income` for life-insurance payouts".
- [ ] 6.2 Add a "Risk management / insurance modeling" section
  to the skill with two worked examples (life insurance and
  income protection / disability).
- [ ] 6.3 Create `scenarios/insurance-life.yaml` exercising the
  life-insurance pattern.
- [ ] 6.4 Create `scenarios/insurance-disability.yaml`
  exercising the disability pattern.

## 7. Validate

- [ ] 7.1 `cargo test` — all existing tests still pass, all new
  tests pass.
- [ ] 7.2 Run `scenarios/insurance-life.yaml` end-to-end:
  terminal month equals the YAML `death.when`; final cash equals
  pre-payout balance + payout - premiums accrued.
- [ ] 7.3 Run `scenarios/insurance-disability.yaml` end-to-end:
  no terminal month; final cash reflects the lower post-disability
  income.
- [ ] 7.4 Run `scenario.yaml` (no `death` event) and confirm
  byte-identical text output to the pre-change baseline.
- [ ] 7.5 Pipe `cargo run --quiet -- --json scenario.yaml` through
  `jq` and confirm `terminal` is `false`, `months` is the
  unchanged per-month array, and no `terminal_month` key is
  present.