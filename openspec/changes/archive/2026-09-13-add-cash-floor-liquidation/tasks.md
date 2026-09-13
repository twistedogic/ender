## 1. Tests

- [ ] 1.1 Add a regression test asserting the new cash-zero behavior
      with `reserve_months: 0`: a scenario where cash goes negative
      must see funds drawn down to recover. Should FAIL on the
      current code (asserts the new behavior we want) and PASS after
      the code change in 2.1.
- [ ] 1.2 Add a regression test asserting positive cash with no reserve
      does NOT trigger settlement (sanity check on the no-op path):
      rent 2000 from cash 50000 leaves cash at 48000 untouched.
- [ ] 1.3 Add a regression test asserting cash-zero with positive
      reserve restores to the buffer target (not just to zero):
      reserve_months: 3, expense 200, shortfall to 600 buffer.
- [ ] 1.4 Delete `absent_reserve_disables_settlement` (line 1639) —
      asserts the old no-op behavior the change is removing.

## 2. Code

- [ ] 2.1 Delete the `if self.reserve_months == 0 { return; }` block
      at the top of `fn settle` in `src/main.rs` (3 lines, around
      line 763). The target formula below already evaluates to 0 when
      `reserve_months == 0`, so the rest of the function needs no
      change.

## 3. Spec

- [ ] 3.1 Confirm the delta spec at
      `openspec/changes/add-cash-floor-liquidation/specs/decumulation/spec.md`
      matches the three new tests in 1.1–1.3 (one scenario per test).

## 4. Validation

- [ ] 4.1 Run `cargo test` and confirm full suite passes (baseline 93
      tests + 3 new − 1 deleted = 95 expected).