# Proposal: add-cash-floor-liquidation

## Why

ender's `settle()` function already implements the right asset drawdown
order (funds first, then whole-property sales in asset order), but it
only fires when `reserve_months > 0`. With the default `reserve_months: 0`,
cash can go deeply negative while the user still owns the underlying
assets. The current workaround ("set `reserve_months: 6` to fund a
buffer") is indirect: it conflates "don't go negative" with "keep N
months of expenses in cash." A scenario author who only wants the
former has no clean way to express it.

This change decouples the two: `settle()` always fires on `cash ≤ 0`
(the cash-zero floor), and `reserve_months` (when set) adds an optional
buffer on top. Existing scenarios with `reserve_months > 0` behave
identically; scenarios with `reserve_months: 0` gain the missing safety
net for free.

## What Changes

- `settle()` removes the `if reserve_months == 0 { return; }` early-out.
  The trigger becomes `cash < target` where `target = reserve_months ×
  expense_base` (already evaluates to 0 when `reserve_months == 0`).
  Restore target is `max(0, reserve_months × expense_base)`.
- One existing test (`absent_reserve_disables_settlement`) is deleted —
  it asserts the old no-op behavior the change is removing.
- `decumulation` spec gains one requirement that makes the cash-zero
  trigger explicit and clarifies that `reserve_months: 0` is a valid
  "no buffer, just don't go negative" mode.
- Order of liquidation (funds → property) is unchanged.
- **Out of scope**: per-asset liquidity tiers, `liquidatable: bool`
  opt-out, partial property liquidation, surfacing redemptions in the
  TUI. These are open threads from the explore session; each is a
  separate capability if pursued.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `decumulation`: add a requirement that `settle()` fires whenever cash
  drops to zero or below, regardless of `reserve_months`. The existing
  wording "draw from assets to maintain cash at or above [target]" is
  already correct when read with `target = reserve_months × expense_base`
  (target is zero when reserve_months is zero); the new requirement makes
  that case explicit and locks in the behavior with a scenario.

## Impact

- `src/main.rs`: 3-line deletion at the top of `fn settle(&mut self,
  expense_base: f64)` (lines 763-765). No other code changes.
- `src/main.rs` tests: delete `absent_reserve_disables_settlement`
  (line 1639). The three other `settle()` tests
  (`cash_above_target_is_untouched`,
  `reserve_breached_only_when_assets_exhausted`,
  `property_sold_after_funds_exhausted`) still pass unchanged — the
  target formula and draw loop are untouched.
- `openspec/specs/decumulation/spec.md`: one new requirement under
  "Reserve target includes withdrawal" asserting the cash-zero trigger
  with `reserve_months: 0`.
- The ~40 other scenarios with `reserve_months: 0` (the default): stay
  solvent in normal operation; under genuine cash-zero stress now
  auto-liquidate instead of going paper-negative. This is the intended
  fix.
- `skills/ender/SKILL.md`: no change required (doesn't currently
  document the reserve-months workaround).