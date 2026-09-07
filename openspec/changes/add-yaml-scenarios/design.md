# Design: Add YAML Scenarios

## Context

`src/main.rs` contains a hand-built simulation model: `Scenario` (cash, cashflows, assets, events) stepped month by month via `once()`. Scenarios can only be constructed programmatically, and the crate has zero dependencies. The file also does not compile (`E0283` on `.sum()`, plus a move-out-of-borrow on `self.events`).

## Goals / Non-Goals

**Goals:**
- Define scenarios in a YAML file: starting cash + timed events.
- Compile again.
- Smallest possible diff: derive on existing enums, no parallel config layer.

**Non-Goals:**
- Down payments / closing costs on `BuyHome` (behavior unchanged, noted as debt).
- Merging `Job`/`Expense` (identical `apply()`, but harmless and `layoff` targets Salary).
- Output serialization of `Stats` (natural follow-on, serde comes free).
- CLI argument parsing for the file path (only a `load` function + a path constant/arg in `main`).

## Decisions

### 1. `serde_yaml_ng`, not `serde_yaml`
`serde_yaml` is archived (unmaintained since 2024). `serde_yaml_ng` is the maintained drop-in fork. This is the crate's first dependency; `serde` (derive) + `serde_yaml_ng` is the minimum tree for YAML input.

### 2. Derive `Deserialize` directly on domain enums — no `EventDef` layer
The enums mix config and simulation state (`Mortgage.paid` counts payments at runtime), but deserialization only constructs values, so direct derives work. Simulation state gets `#[serde(default)]` and stays out of the file format. A parallel def-then-convert layer would be more code for the same YAML.

### 3. `EventType` internally tagged; `CashflowItem`/`Asset` stay externally tagged
`#[serde(tag = "type", rename_all = "snake_case")]` on `EventType` only. Nested internal tags collide (both levels want a tag key in the same map), so full flattening is rejected. Resulting shape:

```yaml
cash: 20000
events:
  - when: 0
    type: job
    salary: { monthly: 8000, annualized_rate: 0.03 }
  - when: 36
    type: buy_to_let
    property:
      sqrt_ft: 1200
      price_per_sqrt_ft: 300
      capex_per_sqrt_ft: 20
      annualized_rate: 0.03
      mortgage:
        mortgage: { monthly: 1800, period: 240 }
      rental:
        rental_income: { occupancy: 0.85, monthly: 2200, annualized_rate: 0.02 }
```

The `mortgage: mortgage:` doubling (outer field name + external tag of `CashflowItem::Mortgage`) is accepted — renaming to avoid it costs more cleverness than it saves.

### 4. `ScenarioInput` wrapper, not deriving on `Scenario`
`Scenario` carries runtime state (`at`, mutated cashflows). The file-level type is `{ cash, events }` → builds a `Scenario` at month 0. One small struct, keeps `Scenario` un-derived.

### 5. Events fire once and are consumed (drain semantics)
`once()` currently tries to iterate `self.events` by value, which moves out of `&mut self`. Fix: drain events whose `when == at` (e.g. index-based retain loop — `Vec::drain_filter` is unstable). This also defines the semantic: an event with `when: 3` fires at month 3 and never again.

### 6. `CashflowItem::Empty` stays derivable but undocumented
It serializes as `empty: null`. Not worth `#[serde(skip)]` gymnastics; just don't document it in the format.

## Risks / Trade-offs

- [First dependency enters the crate] → Conscious choice; serde is the Rust ecosystem's floor, and `serde_yaml_ng` is maintained.
- [Internally tagged enums buffer content, so field-order/type errors can surface far from their source] → Mitigated by error messages including the file path; spec requires clear failures.
- [`Empty` variant is constructible from YAML] → Harmless (zero cashflow); ignore.
- [f64 for money] → Unchanged; this is a simulation, not accounting. Revisit only if comparing totals for equality.

## Migration Plan

None — no existing users or stored files. Rollback = remove the derives and deps.
