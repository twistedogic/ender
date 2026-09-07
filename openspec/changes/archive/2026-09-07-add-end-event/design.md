## Context

`src/main.rs` (single file, ~450 lines) simulates 360 months from a YAML scenario. Events fire when `when == at`: adders push a `CashflowItem` onto `Scenario.cashflow`, `layoff`/`graduate` remove the *first* item of a hardcoded variant kind, asset events push onto `Scenario.assets`. Assets embed `CashflowItem` directly (`Fund.investment`, `Property.rental`, `Property.mortgage`).

Removal today is first-match-by-kind, which cannot disambiguate two salaries (spouse), overlapping rents, or multiple funds — and nothing can reach asset-embedded items, so "stop this fund's monthly contribution" is inexpressible.

## Goals / Non-Goals

**Goals:**
- Express "this specific cashflow item ends" for any item the author labels.
- Ids work uniformly: top-level cashflows and items embedded in assets.
- Existing scenario files load unchanged (ids optional everywhere).

**Non-Goals:**
- Id uniqueness enforcement at load time (duplicate ids are an author error; removal is deterministic first-match).
- Reducing/scaling an item (budget cuts) — removal only.
- A sell/liquidate event for assets (ending a rental's *income* stays possible via `end` on the rental item; selling the property itself is future work).
- Replacing `layoff`/`graduate`.

## Decisions

1. **Id-based targeting, not kind-based.** `end { id }` vs. hardcoded `end_rent` or generic `end { item: rent }`: kind-based removal cannot say *which* of two salaries ends. Ids cost a label in YAML and solve the disambiguation exactly. Alternatives rejected: hardcoded variants (vocabulary grows per cut), kind fallback in `end` (two targeting mechanisms in one event).

2. **Envelope struct, not per-variant fields.** `Labeled { id: Option<String>, item: CashflowItem }` wraps the item wherever it is stored (`Scenario.cashflow: Vec<Labeled>`, and asset-embedded fields become `Labeled`/`Option<Labeled>`). Seven `Option<String>` copies across every `CashflowItem` variant would touch every match arm for zero benefit. `monthly()` delegates through the envelope; the enum and its growth logic stay untouched.

3. **"End anything labeled" — uniform scan.** `end` scans `cashflow[]` first, then assets in order (for a `Property`: rental then mortgage; a `Fund`: investment), removing the first id match. Alternative (top-level only) was rejected: it leaves fund contributions permanently unendable, and "you can end anything you labeled" is one rule to document versus a per-location exception list.

4. **Silent no-op on missing id.** Same behavior as `layoff` with no salary present. Load-time cross-event validation (id defined before `end` fires) is fiddly with event removal semantics and rejected.

5. **First match on duplicate ids**, documented as author error. Deterministic, precedent-consistent.

6. **`layoff`/`graduate` stay unchanged.** They are narrative sugar for unlabeled kind-based removal; deleting them forces ids into every scenario for zero gain.

7. **Same-month event ordering is file order** (existing event-loop behavior, unchanged by this change).

8. **Id placement in YAML rides next to the item's tag**: `{ type: job, id: partner, salary: {...} }`; asset-embedded items nest it one level down (`investment: { id: 401k, investment: { monthly: 500 } }`). Likely requires `#[serde(flatten)]` of the externally-tagged `CashflowItem` inside the internally-tagged `EventType` — YAML is self-describing so flatten should hold, but this is the riskiest plumbing spot.

   *Outcome (task 1.2): the plain path works — `#[serde(default)] id: Option<String>` + `#[serde(flatten)] item: CashflowItem` inside the internally-tagged `EventType` newtype variants deserializes correctly in serde_yaml_ng. No fallback was needed.*

## Risks / Trade-offs

- [serde flatten of an externally-tagged enum inside an internally-tagged enum may fail or surprise] → Spike this first (task 1); fallback is a nested YAML shape or a small custom `Deserialize` for the adder events.
- [Duplicate ids silently end the wrong item] → Deterministic first-match documented; a load-time lint is a cheap future addition.
- [`end` on a mortgage bypasses its `period` payoff — semantically odd] → Allowed but optional; nothing forces its use.
- [Envelope indirection touches every construction/match site] → Mechanical, compiler-guided; the diff is wider than the feature but shallow.

## Migration Plan

None. Ids are optional; no existing file changes. Rollback is reverting the single-file diff.

## Open Questions

- None blocking. The serde flatten spike (task 1) resolves the only technical unknown.
