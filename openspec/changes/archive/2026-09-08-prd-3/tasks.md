## 1. Argument parsing (TODO 6, input side)

- [ ] 1.1 Write failing tests for a pure `parse_args(&args) -> Result<(path, json_flag), String>`: no args → default path, no flag; `["s.yaml"]` → that path; `["--json"]` → flag with default path; `["--json", "s.yaml"]` and `["s.yaml", "--json"]` → both orderings work; `["--yaml"]` → error naming `--yaml`. Confirm the failure mode is the missing function.
- [ ] 1.2 Implement `parse_args` (first non-`-`-prefixed arg is the path, `--json` recognized anywhere, other `-`-prefixed args rejected); replace `args().nth(1)` in `main` with it, printing the parse error to stderr and exiting 1. Tests go green.

## 2. JSON series (TODO 6, output side)

- [ ] 2.1 Write a failing test for a pure `stats_json(&stats) -> String`: run a scenario 12 months, parse the result with `serde_json`, assert 13 elements, `month` fields 0..=12 in order, and numeric `cash`/`assets_value`/`monthly_cashflow` on every element. Confirm the failure mode is the missing function.
- [ ] 2.2 Add `serde_json` to `Cargo.toml`; derive `Serialize` on `Stats`; implement `stats_json` as `serde_json::to_string` over the `enumerate()`d series. Tests go green.
- [ ] 2.3 Write a failing test pinning "JSON matches the text result": for one scenario, the last JSON element's three values equal the numbers `main`'s summary line formats (test against the same `Vec<Stats>`, then eyeball the binary's two modes once).
- [ ] 2.4 Wire `main`: with the json flag print only `stats_json(&stats)`; without it, keep today's summary + insolvent lines byte-for-byte (pin with a test on the text-formatting path if extracted, else by manual run).

## 3. Bookkeeping and verification

- [ ] 3.1 All existing tests stay green (`cargo test`); run the CLI both ways on `scenario.yaml` and confirm: default output unchanged, `--json` output parses with `jq .[-1].cash`.
- [ ] 3.2 Check off TODOS item 6 (item 7 remains for the next iteration).
- [ ] 3.3 `openspec validate prd-3 --strict` passes.
