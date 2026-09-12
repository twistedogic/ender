## 1. Dispatch

- [ ] 1.1 In `src/main.rs::main`, dispatch on the first non-flag
  argument: if it equals the literal `compare`, collect the
  remaining paths and call `compare::run(&paths, json)`; otherwise
  continue with the existing single-scenario path.
- [ ] 1.2 Update `parse_args` (or add a small helper) to return
  either `Mode::Single(path)` or `Mode::Compare(paths)` plus the
  `--json` flag. Preserve the existing error format for unknown
  flags.

## 2. compare module

- [ ] 2.1 Create `src/compare.rs` with a public `pub fn run(paths:
  &[PathBuf], json: bool) -> io::Result<()>`.
- [ ] 2.2 Internally: for each path, load (`load`), simulate
  (`run`), and collect the per-scenario summary (a small struct
  carrying the scenario basename, `KeyStats`, optional terminal
  month, and `goals_outcomes`).
- [ ] 2.3 On any `load` / `into_scenario` / `run` error, return an
  `io::Error` with the path and reason; `main` prints it to stderr
  and exits with code 1.

## 3. Renderers

- [ ] 3.1 Text renderer: aligned columns, two-decimal floats,
  integer months. Output is a single `String` printed once to
  stdout.
- [ ] 3.2 TTY renderer: ratatui table with the same columns; one
  `draw` call then exit (no input loop, no `q` handler — just a
  snapshot). Use `enable_raw_mode` + `EnterAlternateScreen` for the
  duration of the `draw` and restore on exit.
- [ ] 3.3 JSON renderer: emit the array per spec; reuse the wrapper
  object from `add-insurance-modeling` per element with the
  added `name` field.

## 4. Tests

- [ ] 4.1 `dispatch_routes_compare`: `ender compare a.yaml b.yaml`
  reaches the comparison renderer.
- [ ] 4.2 `dispatch_routes_single_scenario_unchanged`:
  `ender a.yaml` and `ender --json a.yaml` continue to produce
  byte-identical output to the pre-change baseline.
- [ ] 4.3 `compare_with_no_paths_fails`: `ender compare` prints
  usage error and exits 1.
- [ ] 4.4 `compare_with_one_scenario_renders_one_row`: a single-row
  table matches the scenario's key stats.
- [ ] 4.5 `compare_with_two_scenarios_renders_two_rows`: row order
  matches the order of paths on the command line.
- [ ] 4.6 `compare_text_format_alignment`: text output's columns
  align to a fixed width; numbers are formatted to two decimals.
- [ ] 4.7 `compare_json_format_shape`: `--json` output parses as
  an array with one element per scenario; each element has the
  expected keys and types.
- [ ] 4.8 `compare_bad_path_errors`: a missing path produces an
  error on stderr and exit code 1, with no TUI entered.
- [ ] 4.9 `compare_unknown_flag_errors`: `--yaml` after
  `compare` produces the same error format as today.
- [ ] 4.10 `compare_handles_death_terminated_scenarios`: a
  scenario with `type: death` shows a terminal-month column value;
  the terminal-month column shows `—` for non-terminated scenarios.

## 5. Documentation

- [ ] 5.1 Add `compare` to the CLI section in
  `skills/ender/SKILL.md` with one worked example
  (`ender compare scenarios/rent-forever.yaml
  scenarios/buy-2027.yaml`).
- [ ] 5.2 Document the dispatch gotcha: a scenario file literally
  named `compare` (no extension) is misinterpreted as the
  subcommand. Recommend naming scenarios descriptively.

## 6. Validate

- [ ] 6.1 `cargo test` — all existing tests still pass, all new
  tests pass.
- [ ] 6.2 Run
  `ender compare scenarios/01-rent-forever.yaml scenarios/02-buy-2027.yaml`
  end-to-end; confirm the table renders with the expected stats.
- [ ] 6.3 Run
  `ender compare --json scenarios/01-rent-forever.yaml scenarios/02-buy-2027.yaml | jq '.[0].key_stats.min_cash'`
  and confirm the value is the same as the corresponding single-
  scenario `--json` run's minimum cash.
- [ ] 6.4 Run `ender scenario.yaml` and confirm byte-identical text
  output to the pre-change baseline (no regression).