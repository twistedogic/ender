## MODIFIED Requirements

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

## ADDED Requirements

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