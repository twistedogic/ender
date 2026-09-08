# Design: add-hk-tax

## Context

`Scenario::once()` in `src/main.rs` runs one month: apply events, iterate
cashflows/assets for monthly amounts, add to cash, then `settle()` restores
the cash reserve by drawing funds and selling properties. There is no calendar
(months are plain `u16` counters from 0) and no taxation. Salary grows
monthly at an annualized rate, so a year's income is the sum of 12 different
monthly amounts, not 12× any snapshot.

HK tax facts baked into this design (IRD 2024/25, verified via IRD/gov.hk):

- Salaries tax = **min**(progressive, standard):
  - Progressive on net chargeable income = gross − MPF − basic allowance
    (132,000; rises to 145,000 in 2026/27):
    50k @ 2%, 50k @ 6%, 50k @ 10%, 50k @ 14%, remainder @ 17%.
  - Standard on net assessable income = gross − MPF: 15% on first 5M,
    16% above.
  - MPF employee contribution: 5% of monthly gross, capped 1,500/month.
- Property tax = 15% × (annual rent × 80%).
- No capital gains or dividend tax; property sale proceeds untaxed.

## Goals / Non-Goals

**Goals:**
- Net-of-tax cash trajectories for salary and rental scenarios.
- Tax charged once per 12-month block, on amounts actually accrued in that
  block (handles mid-year layoffs/graduations for free).
- Configurable basic allowance (the one parameter the government actually
  moves).
- Tax interacts with reserve/settle exactly like any other cash outflow.

**Non-Goals:**
- Calendar alignment (Apr–Mar) or payment-in-arrears timing.
- Married/child/dependent allowances, home-loan-interest deduction,
  charitable donations, personal-assessment election.
- MPF tracked as a redeemable asset.
- Tax on fund growth, dividends, or property sales (HK does not levy these).
- One-off government "tax rebates" (e.g. 2025/26 3,000 cap).

## Decisions

### D1: Accrual counters on `Scenario`, not event snapshots
`Scenario` gains `salary_accrued: f64` and `rent_accrued: f64`. During the
monthly iteration, salary cashflow values and rental income values are added
to the counters. Rationale: `monthly()` already mutates growth rates, so the
returned value is the only correct per-month figure; recomputing a year from
a final-month snapshot would be wrong (and more code). Mid-block removals
(`layoff`, `end`) naturally yield partial-year accrual — correct behavior
for free.

*Alternative rejected*: a separate `TaxLot` per cashflow item — more state,
same answer in this sim.

### D2: Sim-year blocks, tax charged at block end
Tax fires when `(at % 12) == 11`, i.e. months 11, 23, 35…, covering the
trailing 12 months. The sim has no calendar; real HK timing (Apr–Mar, paid
in arrears) adds precision the model can't express. Note `run(n)` executes
`n+1` months (0..=n), so month 360 lands mid-block; any trailing partial
block is simply untaxed — a bounded, documented simplification.

### D3: MPF as a pure salary reduction
`Salary::monthly()` returns `gross − min(gross × 5%, 1500)`. The deduction
vanishes (conservative: money neither accumulates nor returns). MPF is also
the tax deduction, so `mpf_accrued` must be tracked alongside
`salary_accrued` (annual cap 18,000 falls out of the monthly cap).

*Alternative rejected*: contributing MPF into a fund asset — realistic but
adds an asset lifecycle for ~5% of salary; YAGNI.

### D4: Tax is a plain cash outflow inside the settle path
At month 11/23/…, after monthly cashflows are added, subtract
`salaries_tax + property_tax` from `cash` **before** `settle()`. The tax
bill then participates in reserve logic like rent does: it can trigger fund
draws or property sales. Ordering (tax before settle) is pinned by a test.

### D5: One config knob, constants elsewhere
`scenario.yaml` gains an optional `tax.basic_allowance: f64` (default
132,000). Brackets, rates, MPF cap, and the 80% rental factor are consts.
Rationale: brackets change roughly never; the allowance is the realistic
calibration knob (132k → 145k in 2026/27).

## Risks / Trade-offs

- [Existing scenarios get poorer] → Intended effect; but MPF quietly shrinks
  monthly salary cashflow even in zero-tax blocks — flagged BREAKING in the
  proposal, covered by an explicit test.
- [Annual lump distorts reserve rhythm] → Real (HK bills annually too);
  document that month-11 cash dips are expected. If resented later, a
  monthly escrow (1/12 accrual) is a local change.
- [Float drift in tax on grown salary] → Acceptable; sim already compounds
  floats monthly. Worked-example tests use rate 0.
- [Bracket/allowance staleness over years] → `basic_allowance` override plus
  one consts block to update; ceiling documented in code.

## Migration Plan

Additive to the YAML schema (optional `tax` block). Rollback = remove block /
revert commit; no persisted state.

## Open Questions

None blocking. Arrears timing and MPF-as-asset are parked Non-Goals.
