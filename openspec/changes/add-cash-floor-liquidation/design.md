## Context

`ender/src/main.rs` defines `Scenario::settle(&mut self, expense_base:
f64)` at line 762. The function currently early-exits when
`reserve_months == 0`, then computes a target as
`reserve_months × expense_base`, returns if `cash ≥ target`, and otherwise
draws fund principals then sells whole properties until `cash ≥ target`
or assets are exhausted.

`settle()` is called once per simulated month from `Scenario::once()`
(line 750), after all events, cashflows, and the annual tax charge have
been applied, and before the `min_cash` snapshot is recorded at month end.
The order is fixed: events → cashflows → tax → settle → stats snapshot.

The reserve machinery predates the `decumulation` change
(see `add-portfolio-withdrawal-phase` in the archive). That change made
`reserve_months` the canonical way to trigger drawdown for
post-retirement spending; this change makes it also the canonical way to
prevent paper-negative cash in any scenario, regardless of
`reserve_months`.

## Goals / Non-Goals

**Goals:**

- Decouple "don't go cash-negative" from "keep N months of expenses in
  cash." Either should be expressible independently.
- Preserve the existing asset drawdown order (funds → property).
- Preserve all existing tests except the one that explicitly asserts the
  no-op behavior we're removing.
- Make the new behavior spec-locked so future refactors don't
  accidentally re-introduce the `reserve_months == 0` short-circuit.

**Non-Goals:**

- Per-asset liquidity ordering. The asset enumeration order is still
  `Vec<Asset>` insertion order; `Fund` and `Property` are not
  interleaved by a configurable priority.
- A `liquidatable: bool` opt-out for assets (e.g., "this property is a
  primary residence, never sell").
- Partial property liquidation (selling N sqft of a property to cover a
  shortfall instead of the whole thing).
- Surfacing redemption events to the user (TUI line "redeemed 5000 from
  fund this month"). Settle remains a silent balance-sheet move.
- Changing the `settle()` signature, its callers, or its position in the
  monthly loop.

## Decisions

### D1: Delete the early-out, don't refactor settle() into two functions

The current code:

```rust
fn settle(&mut self, expense_base: f64) {
    if self.reserve_months == 0 {
        return;
    }
    let target = self.reserve_months as f64 * expense_base;
    if self.cash >= target {
        return;
    }
    // ... draw funds then sell property ...
}
```

becomes:

```rust
fn settle(&mut self, expense_base: f64) {
    let target = self.reserve_months as f64 * expense_base;
    if self.cash >= target {
        return;
    }
    // ... draw funds then sell property ...
}
```

When `reserve_months = 0`, `target = 0.0` and the existing draw loop
restores cash to `≥ 0.0` — the natural floor.

*Alternative rejected*: splitting into `try_settle_to_floor()` (cash ≤ 0
trigger) and `try_settle_to_buffer()` (cash < reserve target trigger).
Two functions for one shared draw loop is the speculative flexibility
the prior `add-portfolio-withdrawal-phase` design warned against
("Two liquidation paths in one simulator is exactly the kind of
'speculative flexibility' `AGENTS.md` warns against"). The draw loop
itself is identical in both cases — funds then properties, in asset
order — so the split adds code without changing semantics.

### D2: Don't add a new liquidation_policy block to the schema

A scenario author could today declare
`liquidation: {min_cash_floor: 0, reserve_target: 6 × expense_base,
order: [fund, property]}` and pick exactly what they want. We don't.

Reasons:

- The existing knobs already cover both behaviors. `reserve_months: 0`
  = floor only. `reserve_months: N > 0` = floor + buffer. No scenario
  author needs to express anything else today.
- The two behaviors live in the same number (`reserve_months`). A
  separate config block is two knobs where one sufficed; the second is
  always implied by the first.
- Schema additions are hard to take back. The block would have to be
  optional, and every future reader would have to learn its semantics.
  Prefer to add when a scenario actually needs it (YAGNI per
  `AGENTS.md`).

*Alternative rejected*: a `liquidate_when: cash_zero | reserve_target
| both` field on the scenario. Three-valued enum when today the binary
choice is encoded in `reserve_months`. Adding a third option to a knob
nobody has complained about is speculative flexibility.

### D3: Don't fire a liquidation event

`settle()` mutates `self.cash` and `self.assets` directly. No
synthesized "I sold X from fund Y" event is recorded. This is
unchanged by the proposal — out of scope — and worth flagging because
the TUI cannot today distinguish "this scenario went cash-negative and
auto-liquidated" from "this scenario ended solvent." If that
visibility matters later, it's a separate capability (probably under
`tui-display`).

## Risks / Trade-offs

- **[Scenarios that relied on cash going negative for stress testing
  silently change behavior]**. Scenarios that currently paper-cash
  deeply negative will auto-liquidate to cover instead. Total wealth is
  unchanged; cash trajectory is much higher.

- **[Test coverage of the no-op path is now gone]**. We had one test
  (`absent_reserve_disables_settlement`) asserting the old behavior.
  The new spec replaces it with three scenarios (cash-zero trigger,
  positive-cash no-op, cash-zero with buffer). → Mitigation: the spec
  scenarios are the new regression tests; implementer translates them
  into unit tests under `#[cfg(test)]` in `main.rs`.

- **[Edge case: cash lands at exactly 0 after a draw]**. If the
  simulator draws exactly enough to bring cash from `−100` to `0`,
  the next month starts at 0 and any positive inflow pushes cash
  above 0 without further settling. If the next month has a further
  outflow, cash dips below 0 and settle fires again. This is the
  natural "floor" semantic and matches the user's request
  ("sell off asset if cash hits zero") — no risk.

- **[Floating-point dust at the boundary]**. If a draw brings cash to
  `1e-15` instead of `0.0`, the `cash >= target` check
  (`1e-15 >= 0.0` → true) returns, but a follow-up rounding operation
  might re-dip cash. Acceptable — same behavior as today's
  reserve-target path. → Mitigation: not needed; existing test coverage
  shows the current code tolerates this.

## Migration Plan

No schema change. No CLI change. No persisted state. The only
runtime-visible change is:

1. `src/main.rs`: 3-line deletion in `settle()` (the
   `if self.reserve_months == 0 { return; }` block).
2. `src/main.rs`: 1 test deletion (`absent_reserve_disables_settlement`).
3. `openspec/specs/decumulation/spec.md`: 1 new requirement + 3 new
   scenarios (delta spec in this change; promoted at archive).

Rollback: revert the 3-line deletion and the test is back to its old
behavior.

## Open Questions

None blocking. Open threads captured under Non-Goals above can each
become a follow-up change if a scenario motivates them:

- Per-asset liquidity ordering (new capability `asset-liquidity-order`).
- `liquidatable: bool` opt-out (new capability `asset-liquidity-opt-out`).
- Partial property liquidation (new capability
  `asset-partial-liquidation`).
- Liquidation events surfaced to TUI (under existing `tui-display`).