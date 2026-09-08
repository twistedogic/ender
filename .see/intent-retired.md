# Intent: one-off cashflows and liquidity safety in ender

`ender` currently models only recurring cashflows (salary, rent, rental
income, mortgage). The operator wants the simulator to also model one-off
events and to keep the simulated entity solvent.

What to deliver (from `TODOS`, in priority order):

1. **One-off expense cashflow type** — a single non-recurring outgoing
   (e.g. a purchase, a tax bill) at a point in time.
2. **One-off income cashflow type** — a single non-recurring incoming
   (e.g. a bonus, an inheritance) at a point in time.
3. **Cash reserve requirement** — a configurable target of X months of
   expenses held as cash at all times.
4. **Expense payment precedence** — when a payment is due, fund it by
   using cash first, then tapping the fund, then selling property —
   while still honoring the cash reserve requirement (i.e. only dip
   into cash beyond the reserve when the other sources are exhausted,
   or define and document the exact ordering semantics chosen).

Why: real-world scenarios are not purely recurring, and the interesting
question a simulation must answer is "does the entity stay liquid when
a large one-off bill lands?" — which requires one-off cashflows plus a
reserve-aware payment order.

Capabilities touched: `src/main.rs` cashflow model and simulation loop,
scenario YAML schema (`scenario.yaml`), `openspec/specs/scenario-loading`.

Done when: all four TODOS are shipped, checked off in `TODOS`, and no
archived `prd-*` proposal leaves meaningful ground uncovered. Then
retire this loop.

Retired: all four TODOS shipped (prd-1: one-off cashflows; prd-2: reserve requirement and payment precedence with insolvency reporting); remaining steps are cosmetic.
