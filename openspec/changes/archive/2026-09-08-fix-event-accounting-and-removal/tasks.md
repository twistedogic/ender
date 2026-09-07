## 1. Purchase accounting (fix #2)

- [x] 1.1 Add failing tests: `buy_home` deducts `sqft × price_per_sqft` from cash in the buy month (not in monthly cashflow); `investment` deducts `principal`; deduction never repeats in later months
- [x] 1.2 Deduct asset price in `EventType::apply` for `BuyHome`/`BuyToLet` (`s.cash -= a.value()`) and fund `principal` for `Investment`, leaving `monthly_cashflow` untouched
- [x] 1.3 Verify `scenario.yaml` output shifts (fund purchase now costs 5000 cash) and no test regresses

## 2. Targeted removal (fix #3)

- [x] 2.1 Add failing tests: `layoff` with `id: partner` removes only that salary among two; `graduate` with `id: kid-2` removes only that tuition; `layoff` with unknown id is a silent no-op; `layoff` without id still removes the first salary
- [x] 2.2 Change `Layoff`/`Graduate` to carry `Option<String>` id; with id remove the first cashflow item carrying it (no-op if none), without id keep first-match-of-kind removal

## 3. Tuition rename (fix #4)

- [x] 3.1 Add failing test: a `type: tuition` event loads and adds a tuition cashflow
- [x] 3.2 Rename `EventType::Inition` to `EventType::Tuition` (serde tag becomes `tuition`); confirm `type: inition` now fails with the standard unknown-variant error
- [x] 3.3 Update the vocabulary test to include `tuition` among the event types

## 4. Wrap-up

- [x] 4.1 Run the full test suite (`cargo test`) and a manual `cargo run` on `scenario.yaml`
