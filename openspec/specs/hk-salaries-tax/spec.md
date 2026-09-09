## Purpose

Compute Hong Kong salaries tax (progressive vs standard minimum) charged
annually to scenario cash, with the basic allowance configurable per
scenario from 2026/27 IRD household composition (married, children,
parents).
## Requirements
### Requirement: Salary cashflow is net of MPF
The system SHALL deduct the employee Mandatory Provident Fund (MPF)
contribution from every monthly salary cashflow: 5% of that month's gross
salary, capped at HKD 1,500.

#### Scenario: Salary below the MPF cap
- **WHEN** a month's gross salary is HKD 20,000
- **THEN** the salary cashflow is HKD 19,000 (1,000 deducted)

#### Scenario: Salary above the MPF cap
- **WHEN** a month's gross salary is HKD 40,000
- **THEN** the salary cashflow is HKD 38,500 (1,500 capped deduction)

### Requirement: Salaries tax charged annually on accrued gross
At the end of every 12-month block (months 11, 23, 35, ...), the system
SHALL charge Hong Kong salaries tax on gross salary accrued during that
block, deducted from cash. Accrued salary is the sum of the 12 monthly gross
amounts (gross = net cashflow + MPF deducted). The chargeable computation
SHALL be the minimum of:

- Progressive tax on net chargeable income = accrued gross − accrued MPF −
  basic allowance, using brackets 50k @ 2%, 50k @ 6%, 50k @ 10%, 50k @ 14%,
  remainder @ 17%;
- Standard-rate tax on net assessable income = accrued gross − accrued MPF,
  at 15% on the first HKD 5,000,000 and 16% above.

#### Scenario: Progressive example with IRD-published figures
- **WHEN** a 12-month block accrues gross salary of HKD 350,000 (MPF 18,000,
  basic allowance 132,000, no growth)
- **THEN** net chargeable income is 200,000 and salaries tax charged is
  HKD 16,000 (min of progressive 16,000 vs standard 49,800)

#### Scenario: Income below allowance
- **WHEN** a block accrues gross salary of HKD 100,000
- **THEN** salaries tax charged is 0

#### Scenario: Partial-year accrual after mid-block layoff
- **WHEN** salary is removed after 6 of 12 months in a block
- **THEN** salaries tax is computed on the 6 accrued months only

### Requirement: Basic allowance is a code constant
The system SHALL default the salaries-tax basic allowance to HKD 132,000,
held as a code constant updated when the Inland Revenue Department (IRD)
revises it (145,000 from year of assessment 2026/27). When the scenario
declares a `tax:` block with `status`, `children`, or `parents` fields,
the system SHALL override the default allowance with a household-derived
total computed from those fields using 2026/27 IRD figures: basic
allowance HKD 145,000 for `status: single` or HKD 290,000 for
`status: married`; child allowance HKD 140,000 per child; dependent
parent/grandparent allowance HKD 55,000 per parent in the `60+` age band
and HKD 27,500 per parent in the `55-59` age band, doubled to HKD 110,000
or HKD 55,000 respectively when `living_with: true`. The household
allowance total SHALL be passed to `salaries_tax` in place of the code
constant for the entire run. When the `tax:` block is absent or empty,
the system SHALL use the code constant, preserving prior behavior.

#### Scenario: Allowance revision
- **WHEN** the IRD revises the basic allowance and the constant is updated
- **THEN** all scenarios without a `tax:` block use the new allowance on
  the next run

#### Scenario: Married with two children and two non-cohabiting parents aged 60+
- **WHEN** a scenario declares `tax: { status: married, children: 2,
  parents: [{ age_band: "60+", living_with: false },
  { age_band: "60+", living_with: false }] }`
- **THEN** the salaries-tax allowance total is HKD 680,000
  (290,000 + 2 × 140,000 + 2 × 55,000), and the system uses that figure in
  place of the code constant for `salaries_tax`

#### Scenario: Single with one cohabiting parent aged 55–59
- **WHEN** a scenario declares `tax: { status: single, parents:
  [{ age_band: "55-59", living_with: true }] }`
- **THEN** the salaries-tax allowance total is HKD 200,000
  (145,000 + 55,000)

#### Scenario: Empty tax block falls back to the code constant
- **WHEN** a scenario declares `tax: {}` or omits `status` / `children` /
  `parents` entirely
- **THEN** the salaries-tax allowance is the code constant (132,000 by
  default), identical to a scenario without a `tax:` block

### Requirement: Accrual counters reset each block
Salary and MPF accrual counters SHALL reset to zero immediately after each
annual tax charge.

#### Scenario: Second block taxed independently
- **WHEN** two consecutive 12-month blocks accrue identical salaries
- **THEN** the tax charged in each block is identical

### Requirement: Tax outflow precedes reserve settlement
The annual tax charge SHALL be applied to cash before the reserve
settlement step in the same month, so that a tax bill can trigger fund draws
or property sales to restore the reserve.

#### Scenario: Tax bill drains cash below reserve
- **WHEN** a month-11 tax charge pushes cash below the reserve target
- **THEN** settlement draws from funds (and sells properties only if funds
  are insufficient) in that same month

### Requirement: Tax block fields are validated
The `tax:` block SHALL reject unknown fields with a loading error naming
the file. Each `parents` entry SHALL require both `age_band` and
`living_with`. `status` SHALL be one of `single` or `married`. `children`
SHALL be a non-negative integer.

#### Scenario: Unknown tax field fails to load
- **WHEN** a scenario declares `tax: { shoe_size: 42 }`
- **THEN** loading fails with an error naming the file and the unknown
  field

#### Scenario: Parent entry missing age_band fails to load
- **WHEN** a scenario declares
  `tax: { parents: [{ living_with: false }] }`
- **THEN** loading fails with an error naming the file

