## 1. Tax math (pure functions + tests first)

- [x] 1.1 Add consts (brackets, 15%/16% standard, 132,000 allowance, MPF
  5%/1,500 cap, 80% rental factor) and pure fns `mpf_monthly(gross)`,
  `salaries_tax(gross, mpf, allowance)`, `property_tax(rent)` with tests
  pinning the IRD worked examples from the specs (350k → 16,000; 100k → 0;
  300k rent → 36,000; cap cases).
- [x] 1.2 Test the `basic_allowance` override path (320k gross, 145k
  allowance → chargeable 157,000).

## 2. Accrual + MPF (breaking behavior)

- [x] 2.1 Change `Salary::monthly()` to return `gross − mpf_monthly(gross)`
  with a failing-then-passing test for below-cap and above-cap salaries.
- [x] 2.2 Add `salary_accrued`, `mpf_accrued`, `rent_accrued` to `Scenario`;
  in `once()` accumulate gross (= net + MPF), MPF, and rental-income values
  during the existing iteration.

## 3. Annual charge

- [x] 3.1 At `at % 12 == 11`: subtract `salaries_tax + property_tax` from
  `cash` BEFORE `settle()`, then zero all accrual counters. Tests: identical
  consecutive blocks; partial-block layoff; combined salary+rent bill.
- [x] 3.2 Test tax-bill-drains-reserve ordering (tax first, then fund draw /
  property sale) per the spec scenario.

## 4. Config + docs

- [x] 4.1 Optional `tax.basic_allowance` in `ScenarioInput` → `Scenario`.
- [x] 4.2 Add `tax` block to `scenario.yaml`, note the BREAKING MPF/net
  salary shift in the TODOS/README if present.
