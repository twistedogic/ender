# Tasks: Add YAML Scenarios

## 1. Compile fix (prerequisite)

- [x] 1.1 Fix `E0283` in `Scenario::once`: annotate the `.sum()` calls as `f64`
- [x] 1.2 Fix the move-out-of-borrow on `self.events`: fire events with `when == at` by draining them (index-based retain loop), so each event fires exactly once
- [x] 1.3 Verify `cargo check` passes

## 2. Dependencies and derives

- [x] 2.1 Add `serde` (derive feature) and `serde_yaml_ng` to `Cargo.toml`
- [x] 2.2 Derive `Deserialize` on `CashflowItem`, `Asset`, `EventType`, `Event` with `rename_all = "snake_case"`; internally tag `EventType` with `tag = "type"`; add `#[serde(default)]` to `Mortgage.paid`

## 3. Scenario loading

- [x] 3.1 Add `ScenarioInput { cash, events }` with a `load(path) -> Result<Scenario, ...>` that reads the file, parses YAML, and builds a month-0 `Scenario`; errors include the file path and reason
- [x] 3.2 Wire `main` to load a scenario file path (argument or constant) — stub a minimal example file to load

## 4. Verification

- [x] 4.1 Test the full event vocabulary round-trips: a YAML using every event type loads into the expected `Scenario` (spec: Full event vocabulary)
- [x] 4.2 Test `Mortgage.paid` defaults to 0 when omitted (spec: Mortgage without paid counter)
- [x] 4.3 Test an expense with `when: 3` fires exactly once across a 12-month run (spec: Event does not refire)
- [x] 4.4 Test unknown event type and malformed YAML both fail with errors naming the file (spec: Invalid files fail with a clear error)
