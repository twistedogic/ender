## Context

`ender`'s income model today is two cashflow variants:

- `CashflowItem::Salary { monthly, annualized_rate }` — net of MPF
  (`monthly - mpf_monthly(monthly)`), gross flows into
  `salary_accrued`, MPF flows into `mpf_accrued`, both consumed by
  the annual `salaries_tax` charge in `Scenario::once()` at month 11.
- `CashflowItem::RentalIncome { occupancy, monthly, annualized_rate }`
  — scaled by occupancy, gross flows into `rent_accrued`, consumed by
  `property_tax` (15% on 80% of rent) at the same month-11 charge.

No third "income" variant exists. The `financial-plan` skill treats
pensions as their own column in the client profile (Step 1) and the
distribution-phase income stack (Step 3), with three properties that
salary does not have:

1. **No MPF** — pension is post-retirement; the employee-side MPF
   contribution does not apply.
2. **Indexed to inflation** — many schemes (HK civil service,
   many DB pensions globally) explicitly index to CPI or a wage
   index, separate from the salary growth assumption.
3. **Assessable for salaries tax in HK** — pension is income under
   the IRD's salaries tax regime and stacks on top of any residual
   earned income for the purpose of the annual charge.

Modeling pensions via `salary` would corrupt the MPF computation and
pretend the user is still employed. Modeling them via `rental_income`
would route them through property tax (15% × 80%) instead of
progressive salaries tax. Both are wrong; a third variant is the
right answer.

## Goals / Non-Goals

**Goals:**

- A `Pension` variant with the same shape as `Salary` (`monthly`,
  `annualized_rate`) but no MPF deduction in `monthly()`.
- Pension gross flows into `salary_accrued` for the annual
  salaries-tax charge, but NOT into `mpf_accrued`.
- A `pension` event that adds the cashflow at its firing month.
- Stoppable via the existing `end: {id: ...}` mechanism.
- Tests covering: pension as positive cashflow, no MPF, taxable
  (accrues into salaries tax), `end` removes it, no impact on
  rent_accrued.

**Non-Goals:**

- Pension contribution modeling (accumulating a pension pot via
  monthly contributions during the working life). A pension is a
  cash inflow post-retirement; accumulation is what the existing
  `investment` + `fund` primitives already cover.
- Mandatory Provident Fund (MPF) lump-sum withdrawal on retirement as
  a separate income source. The current model treats MPF purely as a
  monthly deduction; a future change could add an `mpf_withdrawal`
  event that fires a lump sum. Out of scope here.
- Differentiation between civil-service pensions, MPF derived
  pensions, and private pensions. The model treats all pension
  income the same way (gross income, no MP). HK-IRD-level
  differentiation is the kind of edge-case modeling the project's
  "when in doubt, pessimistic but practical" stance rejects.

## Decisions

### D1: New variant, not a flag on Salary
`CashflowItem::Pension { monthly, annualized_rate }` is its own
variant. Salary and pension differ in two ways (MPF deduction,
default tax treatment), and a flag would couple them. Two variants
keep each branch small and the dispatch clear.

*Alternative rejected*: `Salary { monthly, annualized_rate, mpf:
bool }` — fewer variants, but couples pension income to a
work-emulation code path. The whole point of the change is to keep
pension income from looking like a salary.

### D2: Pension feeds salary_accrued, not a new tax bucket
`tax_accrual()` for `Pension` returns `(monthly, 0.0)` where the
first tuple element is the gross salary-equivalent income (added to
`salary_accrued`) and the second is the rent component (zero). This
matches HK practice: pension is assessable under salaries tax, same
progressive vs standard-rate regime as earned income.

*Alternative rejected*: a new `pension_tax` bucket and a separate
annual charge — duplicates the salaries-tax code path. Pension
income is small relative to the salary base; piggybacking on the
existing regime is correct and one less thing to maintain.

### D3: New event type, not under `job`
`EventType::Pension` mirrors `EventType::Job`: a single-purpose event
that adds one labeled cashflow. Pension is conceptually different from
employment — different MPF treatment, different retirement timing —
so the event vocabulary should reflect that.

*Alternative rejected*: extend `Job` to optionally skip MPF — saves
an event type, costs semantic clarity. `job` reads as "employment";
pension is a separate decision domain.

### D4: `annualized_rate` is the inflation-index, not investment return
The field name matches the existing `Salary`/`Rent`/`Tuition` fields
for consistency, even though pension growth is typically CPI-linked
rather than salary-linked. The model is agnostic: a user who wants
3% pension growth writes `annualized_rate: 0.03`, regardless of whether
that represents CPI, wage growth, or contractual indexation. The
skill documents the typical value range (2.5–3%).

*Alternative rejected*: rename the field to `index_rate` — inconsistent
with the rest of the model.

## Risks / Trade-offs

- [Pension could also be modeled as a lump-sum MPF withdrawal at
  retirement] → Out of scope; `one_off_income` already handles that
  pattern (and the scenario author can use it alongside pension for
  the MPF lump-sum component).
- [Pension tax bracket stacking might over-tax a low pension + no
  salary scenario] → This is correct: HK applies the same progressive
  rates regardless of income source. The existing `salaries_tax`
  logic handles it; no special case needed.
- [No way to model pension contribution during working life] →
  Already covered by `investment` + `fund` with monthly contribution;
  if the user wants the pot to be earmarked as "pension", that's a
  label on the fund's `id`, not a model change.

## Migration Plan

Pure addition. `CashflowItem` gains one variant, `EventType` gains one
variant, `Scenario::cashflow` accepts the new item, the annual
salaries-tax charge picks up the new accrual source. No existing
scenario changes; no CLI changes. Rollback = remove the variant from
the enums and the dispatch in `monthly()`/`tax_accrual()`. No
persisted state.

## Open Questions

None blocking. If a future change wants to model MP-side pension
contributions during the working years (employee + employer), the
right place is to extend `Salary` with an `employer_mpf` field — not
to expand `Pension`. If a future change wants tax-free pension
segments (e.g. the HK's MPF tax-exempt portion of voluntary
contributions), that's a separate capability.