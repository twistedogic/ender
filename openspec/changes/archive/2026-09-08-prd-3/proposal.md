## Why

`ender`'s only output is a single human-formatted summary line (plus an optional insolvency line). The intent's second concern — making the result readable — has a machine-readable half that is still missing: another tool cannot pipe the simulation result without scraping prose. The intent states it directly: "`--json` CLI flag — emit the stats result as JSON on stdout." Without it, the trajectory the simulator computes (360 months of cash, assets, and flows) is locked inside the process.

## Position in the improvement loop

Third iteration (`prd-3`) of the continuous improvement loop on `.see/intent.md`.

- `prd-1` shipped TODOS 1–2: `one_off_expense` and `one_off_income` event types.
- `prd-2` shipped TODOS 3–4: `reserve_months` and reserve-aware payment precedence (funds, then property sales), with insolvency reporting.
- TODO 5 (HK salaries/property tax, net-of-MPF salary) shipped outside the loop via `2026-09-08-add-hk-tax`.

This change ships TODO 6: a `--json` flag that emits the per-month stats series as JSON on stdout. Remaining for a later iteration: TODO 7 (ratatui TUI when `--json` is not passed) — deliberately not bundled, so the TUI gets its own focused iteration and the JSON shape it will coexist with is pinned first.

## What Changes

- New CLI flag `--json`. It may appear before or after the scenario path; the first non-flag argument is the path (default `scenario.yaml`), exactly as today. An unrecognized flag (any `-`-prefixed argument other than `--json`) fails with a clear error on stderr and exit code 1.
- With `--json`, `main` prints the full stats series to stdout as one compact JSON array — one object per simulated month, in order: `{"month": M, "cash": C, "assets_value": A, "monthly_cashflow": F}`. Nothing else is written to stdout; load errors keep going to stderr with the same exit code as today.
- Without `--json`, output is byte-for-byte today's (summary line, then the insolvent line when one exists) — TODO 7's TUI will replace this later, not here.
- Insolvency is not duplicated into the JSON: it is derivable by the consumer as the first month whose `cash` is below zero, using the same rule `main` prints today.

## Capabilities

### New Capabilities

- `cli-output`: how the CLI takes its arguments (path, `--json`) and what it writes to stdout — the JSON series contract (fields, ordering, month indexing, stdout/stderr split).

### Modified Capabilities

_(none)_ — `scenario-loading`, `hk-salaries-tax`, and `hk-property-tax` are untouched; the flag changes no simulation semantics.

## Impact

- `src/main.rs`: `Stats` derives `Serialize`; a small pure `parse_args` replaces the raw `args().nth(1)`; `main` gains one branch that maps `stats.iter().enumerate()` into the JSON objects and prints `serde_json::to_string` of the series. No structural changes.
- `Cargo.toml`: adds `serde_json` (one new dependency — correct number formatting and valid JSON on non-finite floats, where a hand-rolled formatter would emit garbage).
- `scenario.yaml`: unchanged.
- `TODOS`: item 6 checked off (item 7 remains open for the next iteration).
- Tests: failing-first tests for the JSON series (length, month indexing, final-month values matching the text path) and for `parse_args` (flag positions, default path, unknown flag), in `src/main.rs`.
