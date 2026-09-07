## Why

The event vocabulary can add cashflows but can only remove them through two hardcoded events (`layoff` removes the first `Salary`, `graduate` removes the first `Tuition`). Scenarios with more than one item of a kind — two salaries (spouse), overlapping rents, multiple funds — cannot say *which* item ends. Stopping an ongoing fund contribution is impossible to express at all, because contributions live inside assets that no removal event reaches.

## What Changes

- New `end` event type: `{ when, type: end, id: <string> }` removes the cashflow item labeled with that id.
- Cashflow items gain an optional `id` label, accepted wherever an item appears:
  - top-level cashflow events (`job`, `expense`, `inition`)
  - inside assets: a fund's `investment` contribution, a property's `rental` income, a property's `mortgage`
- `end` scans the top-level cashflow list, then assets in order, and removes the first item whose id matches. No match is a silent no-op (same behavior as `layoff` with no salary).
- `layoff` and `graduate` are unchanged — kind-based narrative sugar for unlabeled items.
- No breaking changes: `id` is optional everywhere and existing scenario files load unchanged.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `scenario-loading`: event vocabulary gains the `end` type; cashflow items accept an optional `id` label in the file format; new requirement for id-targeted removal semantics (scan order, first match, silent no-op).

## Impact

- `src/main.rs`: `EventType` gains an `End { id }` variant; cashflow items stored in a labeled envelope (`Labeled { id, item }`) in `Scenario.cashflow` and in asset-embedded item fields (`Fund.investment`, `Property.rental`, `Property.mortgage`); `apply()` gains the removal scan.
- `scenario.yaml`: unchanged (ids optional).
- Tests: new cases for end-by-id at each storage site, unknown-id no-op, and backward compatibility of unlabeled scenarios.
