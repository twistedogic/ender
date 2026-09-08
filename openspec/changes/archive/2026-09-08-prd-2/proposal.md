## Why

`ender` can now take a large one-off bill, but it cannot answer the intent's core question: "does the entity stay liquid when one lands?" Cash is allowed to run silently negative, no reserve is maintained, and assets are never tapped to cover a shortfall. The intent's remaining TODOS — cash reserve requirement and reserve-aware payment precedence — close exactly that gap.

## Position in the improvement loop

Second iteration (`prd-2`) of the continuous improvement loop on `.see/intent.md`. `prd-1` shipped TODOS 1–2 (`one_off_expense`, `one_off_income`); the base event model came from earlier non-loop changes (`2026-02-13-add-yaml-scenarios`, `2026-09-07-add-end-event`, `2026-09-08-fix-event-accounting-and-removal`). `prd-1` deliberately deferred TODOS 3–4 together because the payment precedence is defined relative to the reserve. This change ships both: TODO 3 (cash reserve requirement) and TODO 4 (expense payment precedence).

## What Changes

- New optional top-level YAML field `reserve_months` (non-negative integer, default `0`). `0` or absent means no reserve requirement and behavior is exactly today's.
- When a reserve is configured, every month after net flows are applied, the target is `reserve_months ×` that month's recurring expense total. One-off amounts do not count toward the expense base (they are paid, not kept).
- If cash is below target after the month's flows, the shortfall is funded by liquidation in a fixed order: **funds first** (partial drawdown of fund principal, in asset order), **then whole-property sales** (in asset order; proceeds = current market value minus the mortgage's remaining total payments, which clears the debt with the sale). Liquidation stops as soon as cash reaches the target.
- The reserve is breached — cash held below target, possibly negative — only when funds and properties are exhausted. A month that ends with cash below zero after full liquidation is reported as insolvency.
- Liquidation is a balance-sheet move: proceeds and drawdowns never appear in `monthly_cashflow` (same precedent as asset purchase prices).
- `main` prints the first insolvent month when one exists, so the liquidity question has a direct answer in the output.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `scenario-loading`: new requirements for the `reserve_months` file field (optional, defaulted) and for reserve-aware settlement (target computation, liquidation precedence, reserve breach and insolvency reporting, no impact on reported flows).

## Impact

- `src/main.rs`: `ScenarioInput` and `Scenario` gain `reserve_months`; `once()` computes the month's recurring expense base in the same pass as the flows, then a new `settle()` step draws funds and sells properties; `main` prints the first insolvent month. No structural changes.
- `scenario.yaml`: gains `reserve_months: 3` (no visible output change — the sample never approaches the target).
- `TODOS`: items 3 and 4 checked off; all four intent items then shipped.
- Tests: one failing-first test per spec scenario, in `src/main.rs`.
