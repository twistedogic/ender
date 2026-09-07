## Context

`src/main.rs` (single file, ~740 lines) simulates 360 months from a YAML scenario. Events fire when `when == at`: adder events (`job`, `expense`, `tuition`) push a `Labeled` cashflow item onto `Scenario.cashflow`; every month each item's `monthly()` contributes a flow. Two items already carry internal state that expires them: `Mortgage` counts `paid` up to `period`, and an ended mortgage becomes `CashflowItem::Empty`, keeping its slot while costing nothing.

## Goals / Non-Goals

**Goals:**

- Express a single non-recurring outgoing or incoming at a point in time (intent TODOS 1 and 2).
- The amount shows up in the firing month's `monthly_cashflow` — the liquidity question is about flows.
- One-offs are ordinary cashflow items: optional `id`, `end` works on them, no special-casing.

**Non-Goals:**

- Cash reserve requirement and payment precedence (intent TODOs 3 and 4) — next iteration; precedence is defined relative to the reserve, so they ship together.
- Recurring-but-finite one-offs ("monthly for N months") — mortgage already models that shape.
- Splitting or scheduling a one-off across months.

## Decisions

1. **Cashflow item with a `fired` flag, not a direct cash mutation in `apply()`.** Alternative: `apply()` does `s.cash -= amount` like the asset-purchase deduction. Rejected: it would hide the hit from `monthly_cashflow`, and the intent's liquidity question is exactly "what does this month's flow look like". The item approach also reuses the whole `Labeled`/`end` pipeline for free. The self-expiring-state pattern is already established by `Mortgage.paid`.

2. **Two event types, one item variant.** `one_off_expense` / `one_off_income` in YAML, one `CashflowItem::OneOff { amount, fired }` internally with a signed `amount`. The intent names two cashflow types; two vocabulary words keep YAML amounts positive like every other monetary field (`monthly` in both salary and rent), while the internal variant stays single. Reusing the existing `expense`/`job` adders with a `one_off` payload was rejected: a "job" containing a one-off bill is vocabulary lying.

3. **Absolute value on apply.** `OneOffExpense` stores `-amount.abs()`, `OneOffIncome` stores `amount.abs()`. A negative `amount` in YAML then behaves identically to its positive counterpart instead of silently flipping a bill into a windfall (double negation). One call per arm, no validation machinery.

4. **Fired one-off keeps its slot, contributing zero** — same precedent as a paid-off or ended mortgage. Removing it from `cashflow` on fire would mean `monthly()` returning a removal signal, a structural change for zero observable benefit.

5. **No interaction with asset purchase deductions.** Those subtract price from cash without touching `monthly_cashflow` (spec: "one-time deductions SHALL NOT appear in the reported monthly cashflow"). One-offs differ on purpose: an asset purchase converts cash into an asset (value moves, net worth unchanged); a one-off bill is a genuine flow out. The spec states both rules; no code is shared and none should be.

## Risks / Trade-offs

- Vocabulary grows by two event types; every future "event vocabulary" enumeration (specs, docs) must list them. Accepted: symmetric with `job`/`expense`, and a single overloaded `one_off { direction }` event would push direction into a payload field for no gain.
- `fired` is deserialized from YAML if present (`serde(default)` only defaults absence). A scenario hand-writing `fired: true` gets a permanently silent item. Accepted — same exposure exists for `Mortgage.paid` today, and neither is a documented field.
