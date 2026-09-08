## ADDED Requirements

### Requirement: Scenario may declare a cash reserve
The scenario file SHALL accept an optional top-level `reserve_months` field: a non-negative integer number of months of recurring expenses to hold as cash. When the field is absent or `0`, no reserve requirement SHALL apply and behavior SHALL be identical to a scenario without the field. A `reserve_months` value that is not a non-negative integer SHALL fail loading with the existing file error reporting.

#### Scenario: Reserve loads
- **WHEN** a YAML file specifies `reserve_months: 3` alongside `cash` and `events`
- **THEN** loading succeeds and the scenario carries a 3-month reserve requirement

#### Scenario: Absent reserve disables settlement
- **WHEN** a scenario with no `reserve_months` field runs a month whose flows take cash negative while a fund asset exists
- **THEN** nothing is liquidated and cash simply goes negative, exactly as before this change

#### Scenario: Negative reserve fails to load
- **WHEN** a YAML file specifies `reserve_months: -1`
- **THEN** loading fails with an error naming the file

### Requirement: Settlement restores the cash reserve by liquidation in order
When a reserve is configured, the system SHALL, after applying each month's flows to cash, compute the reserve target as `reserve_months ×` that month's recurring expense total — the sum of all negative flows of the month excluding one-off items' firing amounts, including embedded asset flows such as mortgage payments and fund contributions. If cash is at or above the target, nothing SHALL happen. Otherwise the shortfall SHALL be funded in a fixed order: first by drawing fund principal (partially, in asset order, each fund floored at zero), then by selling whole properties in asset order. A property's sale proceeds SHALL be its current market value minus its mortgage's remaining total payments (`monthly × (period − paid)` as of that month), and the sale SHALL remove the asset so its mortgage and rental flows stop in later months. Liquidation SHALL stop as soon as cash reaches the target; sale overshoot SHALL remain in cash. Liquidation proceeds and drawdowns SHALL NOT appear in `monthly_cashflow`. The reserve SHALL be breached — cash left below the target, possibly negative — only when all funds and properties are exhausted, and a month that ends with cash below zero after settlement SHALL be reported by the run as insolvent.

#### Scenario: Fund drawn to top up the reserve
- **WHEN** a scenario with `reserve_months: 3`, cash 100, a rent of 200 per month, and a fund with principal 1000 runs its first month
- **THEN** the month's flow of −200 leaves cash at −100, settlement draws 700 from the fund, cash ends at the 600 target, the fund's principal ends at 300, and `monthly_cashflow` is −200

#### Scenario: One-off does not inflate the reserve target
- **WHEN** a scenario with `reserve_months: 3`, cash 8000, a rent of 2000 per month, a fund with principal 10000, and a `one_off_expense` of 5000 firing in month 0 runs that month
- **THEN** the reserve target is 6000 (rent only), settlement draws 5000 — cash ends at exactly 6000 — not the 20000 shortfall a one-off-inflated target would force

#### Scenario: Property sold after funds are exhausted
- **WHEN** a scenario with `reserve_months: 3`, cash 100, a rent of 200 per month, a fund with principal 100, and a property worth 30000 with a mortgage of 100 per month over 12 periods runs its first month
- **THEN** the fund is drawn to zero, the property is then sold for 28900 (30000 value minus the 1100 remaining mortgage payments), cash ends at 28800, the asset list is empty, and the next month's `monthly_cashflow` is −200 — the rent only, the sold property's mortgage flow gone with it

#### Scenario: Reserve breached only when assets are exhausted
- **WHEN** a scenario with `reserve_months: 3`, cash 50, a rent of 200 per month, a fund with principal 100, and no properties runs its first month
- **THEN** the fund is drawn to zero, cash ends at −50 below the 600 target with no assets left, and the run reports that month as insolvent

#### Scenario: Cash above target is untouched
- **WHEN** a scenario with `reserve_months: 3`, cash 50000, and a rent of 2000 per month runs a month whose net flow keeps cash above 6000
- **THEN** no liquidation occurs and assets are unchanged
