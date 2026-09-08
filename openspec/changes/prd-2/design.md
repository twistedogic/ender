## Context

`src/main.rs` (single file, ~830 lines) runs 360 months from a YAML scenario. Each month, `once()` fires due events, calls `monthly()` on every cashflow item and asset (flows plus in-place appreciation/mortgage counters), sums the flows into `monthly_cashflow`, and adds the sum to cash. Nothing checks the result: cash may go negative silently, and assets are never used to cover shortfalls. The intent's TODOS 3–4 add a configurable cash reserve and a funding order, so a settlement step is needed between "flows applied" and "month closes".

## Goals / Non-Goals

**Goals:**

- TODO 3: `reserve_months` — hold X months of recurring expenses as cash at all times when configured.
- TODO 4: a single, documented funding order for shortfalls: cash above target absorbs payments; funds are drawn next; whole properties sold last; the reserve itself is breached only when everything else is exhausted.
- Answer the intent's liquidity question directly: the run reports the first insolvent month.
- TODOS fully drained: intent retires after this change ships.

**Non-Goals:**

- Fractional reserve months (e.g. 2.5) — integer `reserve_months` suffices.
- Minimizing liquidation losses (tax lots, transaction costs, sell-order optimization) — asset-list order is the documented semantics.
- Automatic de-risking policies (pausing fund contributions when below target, rebalancing). Contributions are ordinary expenses.
- Aborting the run on insolvency — the simulation should show the wreckage, not stop at it.

## Decisions

1. **One settlement rule after netting, not per-payment funding.** When `reserve_months > 0`, after `cash += net_flow`: target = `reserve_months ×` this month's recurring expense total; if cash ≥ target, nothing happens; otherwise the shortfall is funded by liquidation (decisions 4–5). Alternative: intercept each expense as it comes due and route it through a funding function. Rejected: flows are already netted before cash moves, and funding the net shortfall produces the identical cash outcome with one code path. The intent explicitly allows "define and document the exact ordering semantics chosen".

2. **`reserve_months` is optional with default `0`; `0` disables settlement entirely.** Absent field → behavior is byte-for-byte today's (no liquidation, negative cash allowed). Alternative: make the field required — breaks every existing scenario file for zero gain.

3. **Expense base = this month's total negative flows, excluding one-off items' firing amounts.** Computed in the same pass that calls `monthly()` (each call mutates, so it must run exactly once per item per month). Includes rent, tuition, mortgage payments, and fund contributions; excludes `OneOff` by a `matches!` check. Alternative: an allowlist of "recurring" item types — more vocabulary to keep in sync for the same number. Rationale: a reserve sized by a bill that was already paid this month would force needless liquidation exactly when liquidity is tightest.

4. **Liquidation order and granularity.** Funds first, drawn partially (`principal -= draw`, floored at zero), in asset order; then whole properties sold in asset order, stopping as soon as cash reaches target. Funds are liquid by nature; property sales are all-or-nothing and may overshoot the target (surplus stays in cash). "First acquired, first sold" — no optimality claim, just documented determinism.

5. **Property sale proceeds = current market value − remaining mortgage face.** The model carries no mortgage interest rate, so under the model's own terms the face value of the remaining debt is `monthly × (period − paid)` (both fields already exist; `paid` is post-payment this month since settlement runs after `monthly()`). The sale removes the asset — its mortgage and rental flows stop, and next month's expense base drops accordingly — and clears the debt. Underwater sales (face > value) yield negative net proceeds and still clear the debt: honest accounting beats a floor.

6. **Liquidation never touches `monthly_cashflow`.** Drawdowns and sale proceeds are balance-sheet moves, same precedent as asset purchase prices (one-time deductions, not flows). The flows that caused the shortfall are already reported in full.

7. **Insolvency = a month that ends with cash below zero after settlement; the run continues and `main` prints the first such month.** `Stats` already carries cash — a scan in `main`, no new field. Alternative: abort at first insolvency — rejected: the point of the simulation is to see what a bill does, including the months after.

## Risks / Trade-offs

- Whole-property granularity overshoots the target when the last sale more than covers the shortfall → surplus simply sits in cash; acceptable at this fidelity, and the spec scenario pins the behavior.
- Expense base includes fund contributions, sizing the reserve conservatively ("keep everything running for X months", including investing). A policy that pauses contributions under stress is deliberately out of scope.
- No transaction costs or taxes on sales → proceeds are optimistic; add a cost knob when a scenario needs it.
- Settlement ordering interacts with appreciation: values and mortgage counters move before settlement, so sale prices are post-appreciation and faces are post-payment. Documented here; spec scenarios pin the arithmetic.
