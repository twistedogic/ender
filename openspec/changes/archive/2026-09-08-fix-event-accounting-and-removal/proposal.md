## Why

The simulator hands out assets for free (nothing is deducted from cash when a home, rental property, or fund is acquired), `layoff`/`graduate` delete whichever salary or tuition they find first instead of the one the author meant, and the event type `inition` is a typo for `tuition`. Fixing accounting honesty and event targeting before any richer output is built on top.

## What Changes

- `buy_home` / `buy_to_let` deduct `sqft × price_per_sqft` from cash at event time
- `investment` deducts `principal` from cash at event time
- `layoff` and `graduate` accept an optional `id`: with it, the matching item is removed; without it, today's first-match behavior is kept (backward compatible)
- **BREAKING**: event type `inition` is renamed to `tuition` (no file in the repo uses `inition`)
- Accepted consequence, decided explicitly: mortgage payments still flow monthly after the full-price deduction, so total outlay for a mortgaged property exceeds its price and cash may run deeply negative on realistic scenarios

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `scenario-loading`: event vocabulary changes (`inition` → `tuition`); asset-acquiring events deduct from cash; `layoff`/`graduate` gain targeted removal by optional `id`

## Impact

- `src/main.rs`: `EventType` variants and `apply`, no structural changes
- `scenario.yaml`: unchanged syntax, but final numbers shift (its fund purchase now costs 5000 cash)
- `openspec/specs/scenario-loading/spec.md`: requirements updated via delta
- Tests: one regression test per fix, written failing first per AGENTS.md
