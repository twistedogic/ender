## Context

`src/main.rs` (single file, ~1340 lines) runs 360 months from a YAML scenario and collects one `Stats { cash, assets_value, monthly_cashflow }` per month into a `Vec`. `main` reads the path with `std::env::args().nth(1)`, prints a one-line summary of the final month, and scans the series for the first month with negative cash to print the insolvency line. There is no argument parsing beyond "first argument is the path" and no machine-readable output. Intent TODO 6 asks for `--json`; TODO 7 (ratatui TUI replacing the text output) is the loop's next iteration after this one.

## Goals / Non-Goals

**Goals:**

- TODO 6: `--json` flag emitting the stats result as JSON on stdout, pipable (`ender --json | jq ...`).
- The JSON contract pinned before the TUI iteration builds against it.
- Text output unchanged in this change — the TUI owns replacing it.

**Non-Goals:**

- A summary object or insolvency field in the JSON — both derive from the series (last element; first `cash < 0`).
- Pretty-printing / `--pretty` — compact one-line output is the machine contract; humans get the TUI (TODO 7).
- Month filtering, field selection, or schema versioning — YAGNI until a consumer asks.
- A general argument parser (clap) — one flag does not justify the dependency or the scaffold.

## Decisions

1. **Emit the raw per-month series, not a summary.** `main` already holds the whole `Vec<Stats>`; mapping `enumerate()` into `{month, cash, assets_value, monthly_cashflow}` and serializing is less code than aggregating a summary, and strictly more information for the consumer. The current text summary is derivable (last element); the reverse is not true. Alternative: mirror the two text lines as a small object — rejected: it throws away the trajectory, which is the thing worth piping.

2. **Explicit `month` field even though array position encodes it.** Self-describing output costs one `enumerate()` and saves every consumer an off-by-one guess. The spec pins 0-based indexing to match `Stats` collection order and the existing insolvent-month reporting.

3. **`serde_json` rather than hand-rolled formatting.** The values are `f64`s that can in principle be non-finite; `serde_json` emits valid JSON (non-finite floats become `null`) where a `format!`-based emitter would produce unparseable garbage. One small, ubiquitous dependency for correctness at a trust boundary.

4. **Manual arg loop: `parse_args` as a pure function.** Scan `args`: `--json` sets the flag; the first non-`-`-prefixed argument is the path; any other `-`-prefixed argument is an error naming the flag. Three branches, no dependency, unit-testable without spawning a process. Alternative: clap — rejected at one flag; revisit if flags multiply (e.g. the TUI iteration adds keybindings flags).

5. **`--json` changes output only, never semantics.** Same run length (360), same load errors on stderr with exit 1, same simulation. The flag selects a serializer for the identical `Vec<Stats>`. Alternative: JSON mode also changing run behavior (e.g. truncating the series) — rejected: two modes must not drift.

## Risks / Trade-offs

- 361 objects on one line is large for a terminal, but it is not for a pipe — and humans are not the audience of `--json` (they get the TUI next iteration).
- Field names are the internal `Stats` snake_case names (`assets_value`, `monthly_cashflow`). Stable, matches the code; renaming for style would break the just-pinned contract for zero function.
- No trailing newline guarantees beyond `println!`'s — standard for line-oriented Unix tools; `jq` and friends accept it.
