# Proposal: add-hk-tax

## Why

The simulator tracks gross salary and untaxed rental income, so every scenario
overstates post-tax wealth. Hong Kong taxes are predictable and formula-driven,
making an accurate net-of-tax simulation cheap to add. All existing scenarios
currently produce optimistic cash and asset trajectories.

## What Changes

- Add Hong Kong salaries tax, charged to cash once every 12 simulated months
  (at month 11, 23, 35, ...), computed on salary **accrued** during the block:
  - Deduct Mandatory Provident Fund (MPF) employee contributions from monthly
    salary cashflow: 5% of monthly gross, capped at HKD 1,500/month (money
    leaves cash; not tracked as an asset).
  - Tax = min(progressive on net chargeable income, standard rate on net
    assessable income).
    - Progressive: first 50k @ 2%, next 50k @ 6%, next 50k @ 10%, next 50k @
      14%, remainder @ 17%.
    - Standard: 15% on first 5M, 16% above.
    - Basic allowance 132,000 (configurable override in scenario.yaml).
- Add Hong Kong property tax on rental income, charged in the same annual
  block: 15% × (accrued rent × 80%).
- Tax cash outflow participates in the existing reserve/settle logic (a tax
  bill can trigger fund draws or property sales, like any other expense).
- No tax on fund growth, dividends, or property sale proceeds (HK does not
  tax these for individuals).
- **BREAKING**: monthly salary cashflow becomes net of MPF (~5% lower);
  scenarios now lose tax annually, lowering cash trajectories.

## Capabilities

### New Capabilities
- `hk-salaries-tax`: Annual Hong Kong salaries tax computation (MPF deduction,
  progressive vs standard rate minimum, basic allowance) and its application
  to scenario cash every 12 months.
- `hk-property-tax`: Annual Hong Kong property tax on accrued rental income
  (15% of 80% of rent) and its application to scenario cash every 12 months.

### Modified Capabilities

(none — no existing specs)

## Impact

- `src/main.rs`: `Scenario` gains annual accrual fields and a tax checkpoint
  in `once()`; `Salary::monthly()` becomes net of MPF; `RentalIncome`
  accrual feeds property tax.
- `scenario.yaml` schema: optional `tax` block (`basic_allowance` override).
- Existing scenarios: results shift (lower cash), no schema break beyond the
  new optional block.
- Tests: regression coverage for tax math (IRD-published worked examples)
  and the tax-vs-reserve interaction.
