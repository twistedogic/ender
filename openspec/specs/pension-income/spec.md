# pension-income Specification

## Purpose

Model recurring pension income as a non-MPF cashflow that is assessable under
Hong Kong salaries tax, so retirement scenarios can carry a post-employment
income stream without abusing `salary` (which would silently deduct MPF).
## Requirements
### Requirement: Pension cashflow is non-MPF recurring income
The simulator SHALL carry a `CashflowItem::Pension { monthly, annualized_rate }`
variant. Its `monthly()` SHALL grow `monthly` by `(1 + annualized_rate)^(1/12)`
per simulated month (the same monthly-growth convention as `Salary`, `Rent`,
and `Tuition`) and SHALL return `(Cashflow::Income(m), 0.0)` where `m` is the
pre-growth monthly — i.e. the full gross lands in cash with no employee-side
Mandatory Provident Fund (MPF) deduction. The pension SHALL be removable by
an `end: {id: ...}` event whose `id` matches the pension's `Labeled.id`,
identical to how a labeled salary is removed.

#### Scenario: Pension grows monthly like salary
- **WHEN** a pension of 10000 with `annualized_rate: 0.12` is active for two months
- **THEN** its `monthly` after the second month equals `10000 * (1.12)^(2/12)`, matching what a 10000 salary with the same rate would show after two months.

#### Scenario: End removes a labeled pension
- **WHEN** a scenario declares a pension event with `id: civ_serv` and a later event `{ type: end, when: 480, id: civ_serv }`
- **THEN** the `Pension` item is removed from the cashflow list at month 480 and the months from 481 onwards carry no pension inflow.

#### Scenario: End on unknown pension id is a no-op
- **WHEN** an `end` event fires with `id: ghost` and no active pension carries that id
- **THEN** nothing is removed, the simulation continues, and no error is reported.

### Requirement: Pension does not contribute to MPF or property-tax accruals
A pension's gross SHALL flow into `salary_accrued` (so it is assessable for
salaries tax; see capability `hk-salaries-tax`) but SHALL NOT add to
`mpf_accrued` and SHALL NOT add to `rent_accrued`. The MPF for the block is
computed from `salary` items only.

#### Scenario: Pension does not inflate MPF accrual
- **WHEN** a scenario with a salary of 20000 (MPF 1000/mo) and a pension of 20000 active for 12 months runs to month 11
- **THEN** `mpf_accrued` is `12 * 1000 = 12000` — the pension's presence does not add MPF.
