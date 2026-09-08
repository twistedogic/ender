# Intent: one-off cashflows, liquidity safety, and result output in ender

`ender` models cashflows over time and reports the resulting entity state.
Two distinct concerns are still open: (1) making the simulation match
real-world HK personal finance (one-off events, reserves, taxes), and
(2) making the result readable (machine-readable for piping, human-readable
otherwise).

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
5. **HK salaries and property tax** — annual tax checkpoint every 12
   months (months 11, 23, 35, ...); MPF deducted from gross salary
   (5% capped at HKD 1,500/month); salaries tax = min(progressive,
   standard rate) on net chargeable income; property tax = 15% × 80%
   of accrued rent. Tax bills flow through the same reserve/settle
   path as other expenses. BREAKING: salary cashflow is net of MPF.
6. **`--json` CLI flag** — emit the stats result as JSON on stdout.
7. **Ratatui TUI** — when `--json` is not passed, render an
   interactive TUI showing key stats over the simulation.

Why: real-world scenarios are not purely recurring, the interesting
question a simulation must answer is "does the entity stay liquid when
a large one-off bill lands?" (one-off cashflows + reserve-aware
payment order), HK tax materially shifts long-horizon trajectories,
and the result is only useful if a human can read it (TUI) or another
tool can pipe it (JSON).

Capabilities touched: `src/main.rs` cashflow model and simulation loop,
scenario YAML schema (`scenario.yaml`), `openspec/specs/scenario-loading`,
`hk-salaries-tax`, `hk-property-tax`, CLI argument parsing, TUI rendering.

Done when: all seven TODOS are shipped, checked off in `TODOS`, and no
archived `prd-*` proposal leaves meaningful ground uncovered.

Retired 2026-09-09: all seven TODOs shipped (prd-1: one-off cashflows; prd-2: reserve + payment precedence; add-hk-tax: TODO 5; prd-3: --json; prd-4: TUI) — no meaningful ground left uncovered.
