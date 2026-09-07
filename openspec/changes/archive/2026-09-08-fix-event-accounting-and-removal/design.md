## Context

`ender` is a single-file Rust simulator: YAML scenario → monthly tick over 360 months → one summary line. Assets (property, fund) enter via events; `layoff`/`graduate` terminate cashflows; `end` removes items by id. Three defects: acquisitions never deduct from cash, `layoff`/`graduate` remove by first match rather than by target, and the `Inition` variant is a typo for tuition.

## Goals / Non-Goals

**Goals:**
- Buying a home/rental property deducts `sqft × price_per_sqft` from cash; buying a fund deducts `principal`
- `layoff`/`graduate` remove a targeted item via optional `id`, keeping first-match behavior when no `id` is given
- Rename `Inition` → `tuition` in the event vocabulary

**Non-Goals:**
- Loan balances, amortization, or mortgage interest — mortgage stays a flat payment for `period` months
- Spending `capex_per_sqft` (grows, never spent — known oddity, untouched)
- Trajectory/rich output
- Solvency semantics for negative cash (cash simply accumulates negative)

## Decisions

1. **Full-price deduction at event time, direct to cash.** Deduct in `EventType::apply` (`s.cash -= asset.value()` for properties, `-= principal` for funds), not through the monthly cashflow sum. Alternatives: a `down_payment` YAML field (rejected — user chose full price; zero YAML changes), or deriving an implicit loan from `monthly × period` (rejected — magic number from an interest-free model). Consequence: a mortgaged property's total outlay exceeds its price; accepted.
2. **One-time costs do not appear in `monthly_cashflow`.** `Stats.monthly_cashflow` keeps meaning "recurring flows"; the deduction is visible in `cash` only. Alternative: inject the purchase as a one-shot cashflow item — rejected, pollutes the flow list with single-fire entries.
3. **Optional `id` on `layoff`/`graduate`, mirroring `end` semantics.** With `id`: remove the first cashflow item carrying it, scanning the top-level list; unknown id is a silent no-op. Without `id`: today's first-match-of-kind removal (first salary / first tuition). Alternative: required `id` — rejected, breaks existing YAML for no gain.
4. **Rename `Inition` → `Tuition` now.** serde derives the YAML tag (`type: tuition`) from the variant; v0.1, no file in the repo uses `inition`. `Graduate` keeps its name and still removes tuition items.

## Risks / Trade-offs

- [Realistic scenarios go deeply cash-negative at buy month] → Accepted by decision; visible in stats. Upgrade path: add `down_payment` field, one field + one line.
- [External YAML using `type: inition` breaks] → Renaming while pre-1.0; nothing in-repo uses it.
- [`layoff` with an id that labels a non-salary removes that item] → Same semantics as `end`; the id is the contract.

## Migration Plan

None. `scenario.yaml` needs no syntax change; its output numbers shift because its fund purchase now costs 5000 cash.
