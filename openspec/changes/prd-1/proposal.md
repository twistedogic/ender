## Why

`ender` models only recurring cashflows. Real scenarios are not purely recurring: a tax bill, a purchase, a bonus, an inheritance — single amounts at a point in time. Without them, the simulator cannot answer the intent's core question ("does the entity stay liquid when a large one-off bill lands?"), and the later reserve/precedence work has nothing to hit it with.

## Position in the improvement loop

First iteration (`prd-1`) of the continuous improvement loop on `.see/intent.md`. No archived `prd-*` change exists yet. The foundation was delivered by earlier, non-loop changes: `2026-02-13-add-yaml-scenarios` (YAML loading and the event model), `2026-09-07-add-end-event` (id labels and targeted removal), `2026-09-08-fix-event-accounting-and-removal` (honest asset accounting, `inition` → `tuition`).

This change ships the intent's TODOS **1 (one-off expense)** and **2 (one-off income)**. Remaining for later iterations: TODO 3 (cash reserve requirement) and TODO 4 (reserve-aware payment precedence) — deliberately deferred together, since the precedence ordering is defined relative to the reserve.

## What Changes

- New event type `one_off_expense`: `{ when, type: one_off_expense, one_off: { amount: X } }` — a single non-recurring outgoing at a point in time.
- New event type `one_off_income`: same shape — a single non-recurring incoming.
- A one-off contributes `amount` to the month's cashflow in the month its event fires (shown in `monthly_cashflow`), contributes zero in every later month, and is consumed like any other event.
- Direction comes from the event type: `amount` is given as a positive number in YAML; the sign of a negative `amount` is ignored (absolute value taken), so `-X` cannot silently flip a bill into a windfall.
- One-offs are ordinary labeled cashflow items: optional `id`, removable via `end`, no change needed to existing removal semantics.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `scenario-loading`: event vocabulary gains `one_off_expense` and `one_off_income`; new requirement for one-off semantics (fires once, appears in the firing month's cashflow, zero afterwards, direction from event type); the list of id-labeled top-level events and the runtime-state defaults requirement are extended accordingly.

## Impact

- `src/main.rs`: `CashflowItem` gains a `OneOff { amount, fired }` variant whose `monthly()` self-expires via the `fired` flag (same pattern as `Mortgage.paid`); `EventType` gains `OneOffExpense(Labeled)` / `OneOffIncome(Labeled)`; two new `apply` arms push the item with the sign applied. No structural changes.
- `scenario.yaml`: unchanged (no one-off events used yet).
- `TODOS`: items 1 and 2 checked off.
- Tests: one failing-first test per new event type plus a sign-edge test, in `src/main.rs`.
