## Requirements

### Requirement: Property tax charged annually on accrued rent
At the end of every 12-month block (months 11, 23, 35, ...), the system
SHALL charge Hong Kong property tax on rental income accrued during that
block, deducted from cash. Property tax SHALL be 15% of 80% of accrued gross
rent (the 20% standard repair allowance), summed across all rental income
sources.

#### Scenario: Single rental property
- **WHEN** a block accrues HKD 300,000 of gross rent
- **THEN** property tax charged is HKD 36,000 (300,000 × 0.8 × 0.15)

#### Scenario: Occupancy gaps reduce accrual
- **WHEN** a rental income item has occupancy 0.5 and gross monthly rent
  HKD 10,000 for a full block
- **THEN** accrued rent is HKD 60,000 and property tax is HKD 7,200

#### Scenario: No rental income
- **WHEN** a block accrues no rent
- **THEN** property tax charged is 0

### Requirement: Rent accrual resets each block
Rental accrual counters SHALL reset to zero immediately after each annual
property tax charge.

#### Scenario: Vacant block after tenancy ends
- **WHEN** a tenancy ends mid-scenario and the next block accrues no rent
- **THEN** that block's property tax is 0

### Requirement: Property tax joins salaries tax in the same charge
Property tax and salaries tax for the same block SHALL both be charged in
the same month (block end), as separate components of one cash deduction.

#### Scenario: Combined bill
- **WHEN** a block accrues both salary and rent
- **THEN** the block-end cash deduction equals salaries tax + property tax
  for that block
