# Add YAML Scenarios

## Why

Scenarios are currently hand-constructed in Rust. Defining them in a YAML file makes them editable without recompiling and gives ender its first real user-facing input surface.

## What Changes

- Add a YAML scenario file format: starting cash plus a list of events (job, expense, layoff, buy home, buy to let, investment).
- Parse scenario files with `serde` + `serde_yaml_ng` (the maintained fork; `serde_yaml` is archived).
- Deserialize directly onto the existing `Event`, `EventType`, `CashflowItem`, and `Asset` enums — no parallel config types.
- Add a `ScenarioInput` wrapper (cash + events) that builds a `Scenario`.
- Fix the pre-existing compile errors that block any work in `src/main.rs`:
  - `E0283`: the `.sum()` in `Scenario::once` needs a type annotation.
  - `for e in self.events` moves the events vector out of `&mut self`; events now fire once and are consumed (drain semantics).

## Capabilities

### New Capabilities
- `scenario-loading`: Load a scenario (starting cash + events) from a YAML file into a runnable `Scenario`, with defined error reporting for invalid files.

### Modified Capabilities
<!-- None: no existing specs. -->

## Impact

- `src/main.rs`: serde derives on `Event`/`EventType`/`CashflowItem`/`Asset`, `ScenarioInput`, `Scenario` construction, event-firing loop.
- `Cargo.toml`: first dependencies added — `serde` (derive) and `serde_yaml_ng`.
- Out of scope, noted during exploration: `BuyHome` never deducts a down payment from cash; `Job`/`Expense` have identical apply behavior; `f64` for money.
