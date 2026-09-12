## Context

`ender` is a deterministic Hong Kong cashflow + asset simulator: salary,
rent, mortgage, fund, property, MPF, IRD salaries/property tax. Every
event in a scenario fires at `when: <month>` and the monthly loop in
`Scenario::once()` applies it before walking the cashflow list and
settling the reserve. The model has no notion of "retirement" — the
closest primitives are `one_off_expense` (lump sum) and the `settle()`
function that liquidates funds then properties when cash falls below
`reserve_months × expense_base`.

The `financial-plan` skill's Step 3 Distribution Phase asks questions
the model can't currently answer:

- "If I retire at month 360 with $X in assets, can I sustain $Y/yr
  spending until age 90?"
- "What is the portfolio trajectory post-retirement?"
- "How does a market downturn at retirement (sequence-of-returns risk)
  affect solvency?"

These all need one missing concept: a *recurring* cash outflow that
starts at a chosen month and is automatically funded from the reserve
machinery when cash is short. The reserve machinery already does the
correct thing (funds first, properties last) — it just needs to be
*triggered* by a drawdown-style expense rather than only by an empty
cash balance.

## Goals / Non-Goals

**Goals:**

- A new `withdrawal` cashflow type that is, at the math layer,
  indistinguishable from `Rent` (monthly + monthly compounding + counted
  in `expense_base`).
- A new `withdrawal` event that adds the cashflow at its firing month
  and stops cleanly via the existing `end: {id: ...}` mechanism.
- A worked example in the skill showing: stop salary at month 360, stop
  rent at month 360 (downsizing), add withdrawal of 30k with 2.5%
  inflation, set `reserve_months: 12`, run the full 360+300 months.
- Tests: the new variant matches `Rent` math (no MPF, no tax
  contribution, identical monthly growth); `expense_base` includes it so
  the reserve target scales; `end` removes it.

**Non-Goals:**

- A new "retirement mode" flag, separate reserve regime, or sequence of
  returns simulation. The reserve machinery is the answer; users size
  `reserve_months` to whatever they want funded.
- Withdrawal sequencing rules (4%-rule, RMD-style tables, dynamic
  withdrawal). The user picks a `monthly` figure and an inflation rate;
  that's the whole decision model.
- Tax on withdrawal income. ender models salaries tax / property tax;
  withdrawal cashflow has no source-of-income attribution and is
  treated as non-taxable (same as `Rent`). If a future change wants to
  tax withdrawals, that's a separate capability.
- Withdrawal from a specific asset (e.g., "draw only from this fund").
  Drawdown order is the reserve's job, not the cashflow's.

## Decisions

### D1: New variant, not a flag on Rent
`CashflowItem::Withdrawal { monthly, annualized_rate }` lives alongside
`Rent` and `Tuition` as its own variant. They share the math
(monthly grows at `^(1/12)`, contributes to `expense_base`, no MPF,
no tax accrual) but a separate variant keeps scenarios readable: a
reader scanning a YAML sees `withdrawal: {monthly: 30000}` next to
`rent: {monthly: 20000}` and immediately knows which is the retirement
spend and which is the working-life cost.

*Alternative rejected*: a `kind: withdrawal` field on `Rent` — one
fewer enum variant, but the YAML carries the label semantics
implicitly, which makes the existing scenarios ambiguous. Naming the
variant is the cheaper choice.

### D2: New event type, not under `expense`
`EventType::Withdrawal` mirrors `EventType::Job` (single-purpose event,
carries one map). It does NOT extend `EventType::Expense` because
`expense` already wraps a single inner map (`rent`, `tuition`); adding
`withdrawal` there means another branch in `Expense::item` for what is
conceptually a different domain (drawdown, not living cost).

*Alternative rejected*: extend `Expense` to accept `withdrawal:` —
saves one enum variant, costs semantic clarity. `expense: {withdrawal:
...}` reads as "a withdrawal that's also an expense", which it isn't.

### D3: Reserve machinery is the drawdown trigger — no new logic
`Settle()` already iterates `Scenario::assets`, draws fund principals
to floor zero, then sells whole properties in asset order. Adding
`Withdrawal` to `expense_base` is the entire integration. Users
authoring a retirement scenario set `reserve_months: 12` (or whatever
buffer they want) and let the same code path handle the drawdown.

*Alternative rejected*: a new `drawdown: {start, monthly, rate,
source_order}` top-level block that runs its own liquidation pipeline.
Two liquidation paths in one simulator is exactly the kind of
"speculative flexibility" `AGENTS.md` warns against.

### D4: Withdrawal is not taxable
`tax_accrual()` for `Withdrawal` returns `(0.0, 0.0)` — same as `Rent`.
Pension / annuity income (a separate capability, `add-pension-cashflow`)
is the model for taxable retirement income. Mixing the two would force
the model to track "is this withdrawal from a taxable source", which
is a tax-attribution problem the existing `salary`/`rental` split
already solves cleanly.

*Alternative rejected*: tax withdrawal like salary — pretends the user
hasn't separately modeled pension income, and forces MPF to apply
(retirement income does not attract MPF).

## Risks / Trade-offs

- [No asset-source selection for withdrawal] → Acceptable: the reserve
  machinery handles drawdown order; users who want a custom order can
  model it via explicit `one_off_income` events from a specific fund
  (the existing primitive for that).
- [Withdrawal lump-sum vs monthly] → The model already supports
  monthly (this change) and lump-sum (`one_off_expense`); a 12-month
  prepaid retirement expense is the existing `one_off_expense`.
- [Sequence-of-returns risk is not modeled] → Per `skills/ender/SKILL.md`,
  model downturns explicitly with `downturn` events. The skill's
  instruction is to drop a downturn at the start of retirement, not to
  add a Monte Carlo layer.

## Migration Plan

Pure addition. `CashflowItem` gains one variant, `EventType` gains one
variant, the YAML schema gains one event type. No existing scenario
changes; no CLI changes. Rollback = remove the variant from the enums
and the dispatch in `monthly()`. No persisted state.

## Open Questions

None blocking. If a future change wants tax on retirement income, the
right place is `add-pension-cashflow` (treat the pension as the
taxable source). If a future change wants dynamic withdrawal (4% rule,
guardrails), that's a separate capability and a different `event` kind.