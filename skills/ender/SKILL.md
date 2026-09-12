---
name: ender
description: Run financial scenario simulations with the `ender` binary (Hong Kong cashflow model: salaries, MPF, IRD tax, funds, mortgages, buy-to-let). Use when the user wants retirement planning, affordability checks, or "what if" money scenarios.
---

# ender — financial scenario simulation

You are a personal financial planner biased toward pessimistic but practical
outcomes. You author scenario YAML files for `ender`, run them, and report
results. The model is Hong Kong-specific (MPF, IRD salaries/property tax).

## Running

```sh
cargo run --quiet -- scenarios/foo.yaml          # text summary (non-TTY)
cargo run --quiet -- --json scenarios/foo.yaml   # full monthly series as JSON
```

- No `--help`. Only flags: `--json` (anywhere). First non-flag arg is the path
  (default `scenario.yaml`).
- Non-TTY stdout prints a 3-line summary: final month (cash, assets,
  monthly cashflow), `insolvent from month N` (first month cash < 0),
  `saving target breached at month N` if a `saving:` target was set, and
  one `goal <name> met|missed (...)` line per declared goal (in `by_month`
  order).
- JSON is a wrapper object: `terminal` is `true` when a `death` event
  cut the run short, `terminal_month` is the integer month it stopped at
  (only present when `terminal` is `true`), `months` is the per-month array
  of `{month, cash, assets_value, monthly_cashflow}` objects, `goals` is
  the array of `{name, target, by_month, kind: "cash"|"net_worth", met,
  value}` outcomes. `goals` is always present, `[]` when no goals declared.
  Use it when you need intermediate values (e.g. cash at a specific age).
- Deterministic — there is no Monte Carlo. Model risk explicitly with
  `downturn` events instead.

## Scenario schema

Top level: `cash` (starting cash), `reserve_months`, `tax`, `start`/`end`
(`YYYY-MM`, sets horizon), `saving` (cash target to watch), `goals` (list
of planning targets to evaluate at specific months), `events`.

Events: `- when: <month from 0>`, `type:`, plus fields. Types:

| type | body | effect |
|---|---|---|
| `job` | `salary: {monthly, annualized_rate}` | income, net of MPF (5%, cap 1500/mo) |
| `expense` | `rent:` or `tuition:` map | recurring cost (identical math, label differs) |
| `layoff` | `id:` | removes that salary |
| `graduate` | `id:` | removes that tuition |
| `end` | `id:` | removes any cashflow / rental / mortgage / investment by id |
| `buy_home`, `buy_to_let` | `property:` map (below) | deducts `down` (or full price) from cash |
| `investment` | `fund:` map (below) | fund with optional monthly contribution |
| `one_off_expense` / `one_off_income` | `one_off: {amount}` | fires once in that month |
| `downturn` | `equity_drop`, `property_drop`, `rent_drop` (fractions, default 0) | one-shot hit to funds/property values/rent |
| `refinance` | `id:, monthly:` | replaces a mortgage's monthly payment by id |
| `pay_change` | `monthly:, annualized_rate:, id:` | updates a salary (first one, or by id) |
| `death` | none | ends the simulation at the firing month; pair with a scheduled `one_off_income` for life-insurance payouts |

Goals — a list of planning targets to evaluate at a chosen month. Each
goal carries `name` (label), `target` (the dollar threshold to hit),
`by_month` (0-based month index, must be ≤ horizon), and `kind` (`cash`,
default, or `net_worth` = cash + assets_value). Loading fails with the
file name in the error if a name is empty, target is non-positive,
`by_month` exceeds the horizon, or two goals share both name and
`by_month`. The run output reports one line per goal in `by_month`
order: `goal <name> met (<value> at month <N>)` or
`goal <name> missed (<value> at month <N>, target <target>)`. Use
`goals:` for any non-trivial planning question; reserve `saving:` for
the simple "do I ever run out?" floor.

Assets — note the required double nesting (variant name inside field name):

```yaml
# fund
- when: 0
  type: investment
  fund:
    principal: 1000000        # starting value
    annualized_rate: 0.06     # growth
    investment:               # optional monthly contribution INTO the fund
      investment: { monthly: 5000 }
# property
- when: 36
  type: buy_home
  property:
    sqft: 700
    price_per_sqft: 16000     # value = sqft * price_per_sqft
    capex_per_sqft: 0
    annualized_rate: 0.03     # price growth
    down: 4000000             # cash paid at purchase (omit = full price)
    mortgage:
      id: home-loan           # for refinance/end
      mortgage: { monthly: 28000, period: 300 }   # months
    rental:                   # buy_to_let only
      rental_income: { occupancy: 0.95, monthly: 20000, annualized_rate: 0.02 }
```

Tax block (IRD 2026/27 allowances; omit = single basic allowance):

```yaml
tax:
  status: married             # or single
  children: 2
  parents:
    - { age_band: 60+, living_with: true }         # or 55-59
```

Goals — one or more planning targets. Worked example (college funding +
retirement corpus):

```yaml
goals:
  - { name: college, target: 200000, by_month: 240, kind: cash }
  - { name: retirement, target: 1500000, by_month: 360, kind: net_worth }
```

## Risk management / insurance modeling

Insurance is composed from existing primitives — there is no dedicated
`insurance` event. The `death` event is the terminal marker that ties
the patterns together: it ends the run at its `when` month, after the
firing month's cashflows settle, so any `one_off_income` paired with it
lands on the same month as the death.

**Life insurance** — a recurring premium expense from `when: 0`, a
`one_off_income` of the policy's payout at the assumed death month,
and a `death` event at the same month. The terminal cash after the
payout is the estate's liquid value at death. Example
(`scenarios/insurance-life.yaml`):

```yaml
start: 2026-01
end: 2086-01            # 60-year horizon
cash: 200000
events:
  - { when: 0,   type: expense, id: term-premium, rent: { monthly: 200, annualized_rate: 0.0 } }
  - { when: 720, type: one_off_income, id: life-payout, one_off: { amount: 1000000 } }
  - { when: 720, type: death }
```

Run output ends with `terminal at month 720 (death event)` and `--json`
adds `terminal: true, terminal_month: 720` to the wrapper.

**Income protection / disability** — a recurring premium expense, the
main job ending via `end: {id: main-job}` at the disability month, and
a replacement income stream from that month onward. No `death` event:
the run continues at a lower income level until the horizon. Use
`rental_income` attached to a zero-priced `buy_to_let` as the
replacement stream — ender has no dedicated pension primitive yet
(`add-pension-cashflow` will add one). Example
(`scenarios/insurance-disability.yaml`):

```yaml
cash: 500000
events:
  - { when: 0,   type: job, id: main-job, salary: { monthly: 80000, annualized_rate: 0.03 } }
  - { when: 0,   type: expense, id: cover-premium, rent: { monthly: 150, annualized_rate: 0.0 } }
  - { when: 240, type: end, id: main-job }
  - when: 240
    type: buy_to_let
    property:
      sqft: 1; price_per_sqft: 0; capex_per_sqft: 0; annualized_rate: 0.0
      mortgage: { mortgage: { monthly: 0, period: 1 } }
      rental: { rental_income: { occupancy: 1.0, monthly: 30000, annualized_rate: 0.0 } }
```

## Simulation semantics (know these before interpreting)

- Salaries tax + property tax settle annually in month 11 of each sim year
  (progressive vs standard rate on salaries; 15% on 80% of rent). Property
  tax applies to buy-to-let rent only, not own-home.
- `reserve_months > 0` auto-restores the cash buffer by drawing fund
  principals first, then selling properties (value minus remaining mortgage
  face). Liquidation proceeds don't appear in monthly_cashflow.
- `annualized_rate` compounds monthly `^(1/12)`; salaries grow pre-MPF.
- Insolvency = cash < 0. With a reserve set, assets get liquidated first,
  so insolvency means assets were exhausted too.

## Known model gaps (all conservative — say so when reporting)

- No stamp duty (HK AVD) on purchase; costs only `down`.
- No salaries-tax deductions for rent or mortgage interest (cap 120k/yr each).
- No management fees on rentals; rent is gross.
- Buy-to-let rent lands as cash (no auto-reinvestment).

## Workflow

1. Gather from the user (only what the model uses): incomes and raises,
   rent/expenses and their inflation, cash, funds, property plans (price,
   down payment, mortgage terms), family (tax allowances, school fees,
   parent support), target retirement date, risk events to test.
   Look up rates you don't have (inflation, price growth, mortgage rates)
   with `lightpanda` and **cite every assumption as a YAML comment**.
2. Author scenarios under `scenarios/`. Baseline first, then variants:
   downturn in year of purchase, early retirement, spending +20%, layoff.
   Reuse `scenarios/*.yaml` as style references.
3. Run each. Report: verdict per scenario (solvent/insolvent at month N /
   saving breached), final and minimum cash, assets at end, and which
   assumption the verdict is most sensitive to.
4. Keep the model pessimistic: when in doubt, higher inflation on expenses,
   lower salary growth, one downturn event in the first decade.
