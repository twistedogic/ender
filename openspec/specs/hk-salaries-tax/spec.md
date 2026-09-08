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
revises it (145,000 from year of assessment 2026/27). No scenario-file
override exists; editing the constant is the calibration knob.

#### Scenario: Allowance revision
- **WHEN** the IRD revises the basic allowance and the constant is updated
- **THEN** all scenarios use the new allowance on the next run

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
