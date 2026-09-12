use std::io::IsTerminal;

use serde::{Deserialize, Serialize};

mod compare;
mod tui;

#[derive(Clone, Copy)]
enum Cashflow {
    Income(f64),
    Expense(f64),
}

impl Cashflow {
    fn new(v: f64) -> Self {
        if v > 0.0 {
            return Self::Income(v);
        }
        Self::Expense(-v)
    }

    fn value(self) -> f64 {
        match self {
            Self::Income(v) => v,
            Self::Expense(v) => -v,
        }
    }

    fn add(self, c: Cashflow) -> Cashflow {
        Self::new(self.value() + c.value())
    }
}

// Hong Kong tax parameters (Inland Revenue Department, year of
// assessment 2024/25). ponytail: constants, not config — update when
// the IRD moves them (basic allowance rises to 145,000 in 2026/27).
const BASIC_ALLOWANCE: f64 = 132_000.0;
const MPF_RATE: f64 = 0.05;
const MPF_CAP: f64 = 1_500.0;
const PROGRESSIVE_BRACKETS: [(f64, f64); 5] = [
    (50_000.0, 0.02),
    (50_000.0, 0.06),
    (50_000.0, 0.10),
    (50_000.0, 0.14),
    (f64::INFINITY, 0.17),
];
const STANDARD_RATES: [(f64, f64); 2] = [(5_000_000.0, 0.15), (f64::INFINITY, 0.16)];
const RENTAL_REPAIR_ALLOWANCE: f64 = 0.8;
const PROPERTY_TAX_RATE: f64 = 0.15;

/// Employee Mandatory Provident Fund (MPF) contribution for one month.
fn mpf_monthly(gross: f64) -> f64 {
    (gross * MPF_RATE).min(MPF_CAP)
}

fn bracket_tax(income: f64, brackets: &[(f64, f64)]) -> f64 {
    let (mut tax, mut left) = (0.0, income);
    for &(width, rate) in brackets {
        let take = left.min(width);
        tax += take * rate;
        left -= take;
    }
    tax
}

/// Hong Kong salaries tax on a year's accrual: the lower of progressive
/// tax on net chargeable income and standard-rate tax on net assessable
/// income.
fn salaries_tax(gross: f64, mpf: f64, basic_allowance: f64) -> f64 {
    let nai = (gross - mpf).max(0.0);
    let progressive = bracket_tax((nai - basic_allowance).max(0.0), &PROGRESSIVE_BRACKETS);
    let standard = bracket_tax(nai, &STANDARD_RATES);
    progressive.min(standard)
}

/// Hong Kong property tax: 15% of rent after the 20% repair allowance.
fn property_tax(rent: f64) -> f64 {
    rent * RENTAL_REPAIR_ALLOWANCE * PROPERTY_TAX_RATE
}

/// HK 2026/27 household allowance knobs (IRD year of assessment 2026/27).
/// ponytail: constants — single edit point when IRD revises them.
const MARRIED_BASIC: f64 = 290_000.0;
const SINGLE_BASIC: f64 = 145_000.0;
const CHILD_ALLOWANCE: f64 = 140_000.0;
const PARENT_60_PLUS: f64 = 55_000.0;
const PARENT_55_TO_59: f64 = 27_500.0;

#[derive(Deserialize, Clone, Copy, Default, Debug)]
#[serde(rename_all = "snake_case")]
enum MaritalStatus {
    #[default]
    Single,
    Married,
}

#[derive(Deserialize, Clone, Copy, Debug)]
enum AgeBand {
    #[serde(rename = "55-59")]
    FiftyFiveToFiftyNine,
    #[serde(rename = "60+")]
    SixtyPlus,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ParentConfig {
    age_band: AgeBand,
    #[serde(default)]
    living_with: bool,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
struct TaxConfig {
    #[serde(default)]
    status: Option<MaritalStatus>,
    #[serde(default)]
    children: u32,
    #[serde(default)]
    parents: Vec<ParentConfig>,
}

impl TaxConfig {
    /// Non-empty when any field is present — used to decide whether to
    /// override the default `BASIC_ALLOWANCE`. An empty tax block
    /// (`tax: {}`) is treated as absent.
    fn is_non_trivial(&self) -> bool {
        self.status.is_some() || self.children > 0 || !self.parents.is_empty()
    }
    /// Total allowances derived from household composition (2026/27 IRD).
    /// basic by status (defaults to single), child × 140,000, parents per
    /// age band, doubled when living_with.
    fn annual_basic(&self) -> f64 {
        let basic = match self.status {
            Some(MaritalStatus::Married) => MARRIED_BASIC,
            Some(MaritalStatus::Single) => SINGLE_BASIC,
            None => SINGLE_BASIC,
        };
        let children = self.children as f64 * CHILD_ALLOWANCE;
        let parents: f64 = self
            .parents
            .iter()
            .map(|p| {
                let base = match p.age_band {
                    AgeBand::SixtyPlus => PARENT_60_PLUS,
                    AgeBand::FiftyFiveToFiftyNine => PARENT_55_TO_59,
                };
                if p.living_with { base * 2.0 } else { base }
            })
            .sum();
        basic + children + parents
    }
}

/// Parse `"YYYY-MM"` into `(year, month)` for horizon / saving fields.
/// ponytail: tiny parser — no calendar library, sim has no calendar.
fn parse_ym(s: &str) -> Result<(i32, u32), String> {
    let (y, m) = s.split_once('-').ok_or_else(|| format!("invalid date {s:?}"))?;
    let y: i32 = y.parse().map_err(|_| format!("invalid year in {s:?}"))?;
    let m: u32 = m.parse().map_err(|_| format!("invalid month in {s:?}"))?;
    if !(1..=12).contains(&m) {
        return Err(format!("month out of range in {s:?}"));
    }
    Ok((y, m))
}

/// Months between two `YYYY-MM` dates, inclusive of both endpoints.
fn months_between(start: (i32, u32), end: (i32, u32)) -> Result<u32, String> {
    if end.0 < start.0 || (end.0 == start.0 && end.1 < start.1) {
        return Err(format!("end {end:?} precedes start {start:?}"));
    }
    Ok(((end.0 - start.0) * 12 + (end.1 as i32 - start.1 as i32)) as u32)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum CashflowItem {
    Salary {
        monthly: f64,
        annualized_rate: f64,
    },
    Rent {
        monthly: f64,
        annualized_rate: f64,
    },
    RentalIncome {
        occupancy: f64,
        monthly: f64,
        annualized_rate: f64,
    },
    Mortgage {
        monthly: f64,
        period: u16,
        #[serde(default)]
        paid: u16,
    },
    Investment {
        monthly: f64,
    },
    Tuition {
        monthly: f64,
        annualized_rate: f64,
    },
    OneOff {
        amount: f64,
        #[serde(default)]
        fired: bool,
    },
    Empty,
}

impl Default for CashflowItem {
    fn default() -> Self {
        Self::Empty
    }
}

impl CashflowItem {
    /// Returns (cashflow, recurring_expense_contribution). The expense contribution
    /// counts toward the reserve target; one-offs are excluded by `matches!`.
    fn monthly(&mut self) -> (Cashflow, f64) {
        match self {
            Self::Empty => (Cashflow::Income(0.0), 0.0),
            Self::Investment { monthly } => (Cashflow::Expense(*monthly), monthly.abs()),
            Self::Rent {
                monthly,
                annualized_rate,
            } => {
                let m = *monthly;
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                (Cashflow::Expense(m), m.abs())
            }
            Self::Salary {
                monthly,
                annualized_rate,
            } => {
                let m = *monthly;
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                (Cashflow::Income(m - mpf_monthly(m)), 0.0)
            }
            Self::RentalIncome {
                occupancy,
                monthly,
                annualized_rate,
            } => {
                let m = *monthly;
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                (Cashflow::Income(m * *occupancy), 0.0)
            }
            Self::Mortgage {
                period,
                paid,
                monthly,
            } => {
                if paid >= period {
                    return (Cashflow::Expense(0.0), 0.0);
                }
                *paid += 1;
                (Cashflow::Expense(*monthly), monthly.abs())
            }
            Self::Tuition {
                monthly,
                annualized_rate,
            } => {
                let m = *monthly;
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                (Cashflow::Expense(m), m.abs())
            }
            Self::OneOff { amount, fired } => {
                if *fired {
                    return (Cashflow::Income(0.0), 0.0);
                }
                *fired = true;
                (Cashflow::new(*amount), 0.0)
            }
        }
    }

    /// (salary gross, rent collected) for annual tax accrual. Reads
    /// pre-mutation amounts — call before `monthly()` grows them.
    fn tax_accrual(&self) -> (f64, f64) {
        match self {
            Self::Salary { monthly, .. } => (*monthly, 0.0),
            Self::RentalIncome {
                monthly, occupancy, ..
            } => (0.0, monthly * occupancy),
            _ => (0.0, 0.0),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Labeled {
    #[serde(default)]
    id: Option<String>,
    #[serde(flatten)]
    item: CashflowItem,
}

impl Labeled {
    fn monthly(&mut self) -> (Cashflow, f64) {
        self.item.monthly()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Asset {
    Fund {
        principal: f64,
        annualized_rate: f64,
        investment: Option<Labeled>,
    },
    Property {
        sqft: f64,
        price_per_sqft: f64,
        capex_per_sqft: f64,
        annualized_rate: f64,
        mortgage: Labeled,
        rental: Option<Labeled>,
        /// Cash paid upfront; when set, this is the amount deducted at
        /// purchase time instead of the full `sqft * price_per_sqft`.
        /// Defaults to None (full price deducted, original behavior).
        #[serde(default)]
        down: Option<f64>,
    },
}

impl Asset {
    /// Cash deducted at purchase time: `down` when set, full price otherwise.
    fn purchase_cost(&self) -> f64 {
        match self {
            Self::Property { down: Some(d), .. } => *d,
            _ => self.value(),
        }
    }
    fn value(&self) -> f64 {
        match self {
            Self::Fund { principal, .. } => *principal,
            Self::Property {
                sqft,
                price_per_sqft,
                ..
            } => sqft * price_per_sqft,
        }
    }

    fn monthly(&mut self) -> (Cashflow, f64) {
        match self {
            Self::Property {
                price_per_sqft,
                capex_per_sqft,
                annualized_rate,
                mortgage,
                rental,
                ..
            } => {
                let (rent_income, rent_base) = match rental {
                    Some(r) => r.monthly(),
                    None => (Cashflow::Income(0.0), 0.0),
                };
                let (mort_cf, mort_base) = mortgage.monthly();
                let rate = (1.0 + *annualized_rate).powf(1.0 / 12.0);
                *price_per_sqft *= rate;
                *capex_per_sqft *= rate;
                (rent_income.add(mort_cf), mort_base + rent_base)
            }
            Self::Fund {
                principal,
                annualized_rate,
                investment,
            } => {
                let rate = (1.0 + *annualized_rate).powf(1.0 / 12.0);
                *principal *= rate;
                let (cf, base) = match investment {
                    Some(i) => i.monthly(),
                    None => (Cashflow::Expense(0.0), 0.0),
                };
                if let Cashflow::Expense(v) = cf {
                    *principal += v;
                }
                (cf, base)
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EventType {
    Job(Labeled),
    Expense(Labeled),
    Tuition(Labeled),
    Graduate {
        id: Option<String>,
    },
    Layoff {
        id: Option<String>,
    },
    BuyHome(Asset),
    BuyToLet(Asset),
    Investment(Asset),
    OneOffExpense(Labeled),
    OneOffIncome(Labeled),
    End { id: String },
    /// One-shot percentage drop to funds and/or properties in the firing
    /// month. No cashflow; the post-drop base resumes monthly growth.
    /// `rent_drop` is applied to any property's rental income monthly.
    Downturn {
        #[serde(default)]
        equity_drop: f64,
        #[serde(default)]
        property_drop: f64,
        #[serde(default)]
        rent_drop: f64,
    },
    /// Replace a mortgage's `monthly` payment by id. No-op when no asset's
    /// mortgage matches; missing id fails at load.
    Refinance { id: String, monthly: f64 },
    /// Update a salary's monthly + annualized rate. With `id`, targets the
    /// first cashflow item carrying that id; without, the first salary.
    /// Used for void's `pay-change` events.
    PayChange {
        monthly: f64,
        annualized_rate: f64,
        #[serde(default)]
        id: Option<String>,
    },
    /// Terminal marker for insurance / mortality modeling. No body; sets the
    /// scenario's `terminated` flag at the firing month so the run exits
    /// after the firing month's events and cashflows settle. Pair with a
    /// `one_off_income` at the same `when` to land a payout on death.
    Death {},
}

impl EventType {
    fn apply(self, s: &mut Scenario) {
        match self {
            Self::Job(i) => s.cashflow.push(i),
            Self::Layoff { id } => s.remove_cashflow(
                id.as_deref(),
                |item| matches!(item, CashflowItem::Salary { .. }),
            ),
            Self::Expense(e) => s.cashflow.push(e),
            Self::BuyHome(a) | Self::BuyToLet(a) | Self::Investment(a) => {
                s.cash -= a.purchase_cost();
                s.assets.push(a);
            }
            Self::Tuition(i) => s.cashflow.push(i),
            Self::OneOffExpense(mut i) => {
                if let CashflowItem::OneOff { amount, .. } = &mut i.item {
                    *amount = -amount.abs();
                }
                s.cashflow.push(i);
            }
            Self::OneOffIncome(mut i) => {
                if let CashflowItem::OneOff { amount, .. } = &mut i.item {
                    *amount = amount.abs();
                }
                s.cashflow.push(i);
            }

            Self::Graduate { id } => s.remove_cashflow(
                id.as_deref(),
                |item| matches!(item, CashflowItem::Tuition { .. }),
            ),
            Self::End { id } => {
                if let Some(idx) = s
                    .cashflow
                    .iter()
                    .position(|l| l.id.as_deref() == Some(&id))
                {
                    s.cashflow.remove(idx);
                    return;
                }
                for asset in &mut s.assets {
                    match asset {
                        Asset::Property { rental, mortgage, .. } => {
                            if rental.as_ref().is_some_and(|r| r.id.as_deref() == Some(&id)) {
                                *rental = None;
                                return;
                            }
                            if mortgage.id.as_deref() == Some(&id) {
                                // ended mortgage keeps its slot but stops costing anything
                                mortgage.item = CashflowItem::Empty;
                                return;
                            }
                        }
                        Asset::Fund { investment, .. } => {
                            if investment
                                .as_ref()
                                .is_some_and(|i| i.id.as_deref() == Some(&id))
                            {
                                *investment = None;
                            }
                        }
                    }
                }
            }
            Self::Downturn {
                equity_drop,
                property_drop,
                rent_drop,
            } => {
                for asset in &mut s.assets {
                    match asset {
                        Asset::Fund { principal, .. } => {
                            *principal *= 1.0 - equity_drop;
                        }
                        Asset::Property {
                            price_per_sqft,
                            rental,
                            ..
                        } => {
                            *price_per_sqft *= 1.0 - property_drop;
                            if let Some(r) = rental.as_mut() {
                                if let CashflowItem::RentalIncome { monthly: m, .. } = &mut r.item
                                {
                                    *m *= 1.0 - rent_drop;
                                }
                            }
                        }
                    }
                }
            }
            Self::Refinance { id, monthly } => {
                for asset in &mut s.assets {
                    if let Asset::Property { mortgage, .. } = asset {
                        if mortgage.id.as_deref() == Some(&id) {
                            if let CashflowItem::Mortgage {
                                monthly: m, ..
                            } = &mut mortgage.item
                            {
                                *m = monthly;
                            }
                            return;
                        }
                    }
                }
            }
            Self::PayChange {
                monthly,
                annualized_rate,
                id,
            } => {
                let idx = match id.as_deref() {
                    Some(target) => s
                        .cashflow
                        .iter()
                        .position(|l| l.id.as_deref() == Some(target)),
                    None => s.cashflow.iter().position(|l| {
                        matches!(l.item, CashflowItem::Salary { .. })
                    }),
                };
                if let Some(idx) = idx {
                    if let CashflowItem::Salary {
                        monthly: m,
                        annualized_rate: r,
                    } = &mut s.cashflow[idx].item
                    {
                        *m = monthly;
                        *r = annualized_rate;
                    }
                }
            }
            Self::Death {} => s.terminated = true,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Event {
    when: u16,
    #[serde(flatten)]
    kind: EventType,
}

#[derive(Deserialize, Serialize, Clone, Copy, Default, Debug)]
#[serde(rename_all = "snake_case")]
enum GoalKind {
    #[default]
    Cash,
    NetWorth,
}

#[derive(Deserialize, Debug)]
struct Goal {
    name: String,
    target: f64,
    by_month: u16,
    #[serde(default)]
    kind: GoalKind,
}

#[derive(Serialize, Debug)]
struct GoalOutcome {
    name: String,
    target: f64,
    by_month: u16,
    kind: GoalKind,
    met: bool,
    value: f64,
    /// Month the goal was actually evaluated at. When `add-insurance-modeling`
    /// lands, a death-terminated run evaluates goals past the terminal month
    /// at the terminal month — `value` reflects that month, not the original
    /// `by_month`. JSON output excludes this field; only `by_month` is exposed.
    #[serde(skip)]
    evaluation_month: u16,
}

#[derive(Serialize, Debug)]
struct Stats {
    cash: f64,
    assets_value: f64,
    monthly_cashflow: f64,
}

#[derive(Debug)]
struct Scenario {
    at: u16,
    cash: f64,
    cashflow: Vec<Labeled>,
    assets: Vec<Asset>,
    events: Vec<Event>,
    reserve_months: u32,
    // Accruals since the last annual tax charge (months 11, 23, 35, ...).
    salary_accrued: f64,
    mpf_accrued: f64,
    rent_accrued: f64,
    // Allowance override derived from a non-trivial `tax:` block; None
    // falls back to BASIC_ALLOWANCE.
    allowance_override: Option<f64>,
    // Cash floor from the optional `saving:` field; tracked when present.
    saving_target: Option<f64>,
    // First month (0-indexed) cash dropped below `saving`; None when on
    // target for the entire run. Minimum cash seen during the run.
    saving_breach_month: Option<u16>,
    min_cash: f64,
    // Total months to run: 360 default, or `end - start + 1` when both set.
    horizon: u16,
    // Goals declared on the scenario; evaluated post-run, see `evaluate_goals`.
    goals: Vec<Goal>,
    // Set by `EventType::Death {}` at its firing month. The run exits at the
    // top of the next iteration; the firing month's events and cashflows
    // settle normally so any `one_off_income` paired with the death lands.
    terminated: bool,
}

impl Scenario {
    /// With `id`: remove the first cashflow carrying it (silent no-op if none).
    /// Without: remove the first item matching `of_kind` (legacy behavior).
    fn remove_cashflow(&mut self, id: Option<&str>, of_kind: impl Fn(&CashflowItem) -> bool) {
        let idx = match id {
            Some(id) => self.cashflow.iter().position(|l| l.id.as_deref() == Some(id)),
            None => self.cashflow.iter().position(|l| of_kind(&l.item)),
        };
        if let Some(idx) = idx {
            self.cashflow.remove(idx);
        }
    }

    fn once(&mut self) -> Stats {
        let mut i = 0;
        while i < self.events.len() {
            if self.events[i].when == self.at {
                let event = self.events.remove(i);
                event.kind.apply(self);
            } else {
                i += 1;
            }
        }
        let mut cashflow = 0.0;
        let mut expense_base = 0.0;
        for flow in self.cashflow.iter_mut() {
            let (gross, rent) = flow.item.tax_accrual();
            let (cf, base) = flow.monthly();
            cashflow += cf.value();
            expense_base += base;
            self.salary_accrued += gross;
            self.mpf_accrued += mpf_monthly(gross);
            self.rent_accrued += rent;
        }
        for asset in self.assets.iter_mut() {
            let (_, rent) = match asset {
                Asset::Property { rental: Some(r), .. } => r.item.tax_accrual(),
                _ => (0.0, 0.0),
            };
            let (cf, base) = asset.monthly();
            cashflow += cf.value();
            expense_base += base;
            self.rent_accrued += rent;
        }
        self.cash += cashflow;
        if self.at % 12 == 11 {
            // Charged before settlement so the bill participates in reserve
            // restoration like any other expense.
            let allowance = self.allowance_override.unwrap_or(BASIC_ALLOWANCE);
            self.cash -= salaries_tax(self.salary_accrued, self.mpf_accrued, allowance)
                + property_tax(self.rent_accrued);
            self.salary_accrued = 0.0;
            self.mpf_accrued = 0.0;
            self.rent_accrued = 0.0;
        }
        if self.cash < self.min_cash {
            self.min_cash = self.cash;
        }
        if let Some(target) = self.saving_target {
            if self.cash < target && self.saving_breach_month.is_none() {
                self.saving_breach_month = Some(self.at);
            }
        }
        self.settle(expense_base);
        let assets_value = self.assets.iter().map(|a| a.value()).sum();
        self.at += 1;
        Stats {
            cash: self.cash,
            assets_value,
            monthly_cashflow: cashflow,
        }
    }

    /// Restore the cash reserve by drawing funds, then selling properties.
    /// Liquidation is a balance-sheet move: proceeds never appear in monthly_cashflow.
    fn settle(&mut self, expense_base: f64) {
        if self.reserve_months == 0 {
            return;
        }
        let target = self.reserve_months as f64 * expense_base;
        if self.cash >= target {
            return;
        }
        let need = target - self.cash;
        // Draw fund principals partially in asset order. Funds that hit zero
        // drop out of the asset list (nothing left to settle against).
        let mut drawn = 0.0;
        let mut i = 0;
        while i < self.assets.len() {
            if drawn >= need {
                break;
            }
            if let Asset::Fund { principal, .. } = &mut self.assets[i] {
                let take = (need - drawn).min(*principal);
                *principal -= take;
                drawn += take;
                if *principal <= 0.0 {
                    self.assets.remove(i);
                    continue;
                }
            }
            i += 1;
        }
        self.cash += drawn;
        if self.cash >= target {
            return;
        }
        // Sell whole properties in asset order until target met or assets exhausted.
        // Proceeds = market value - remaining mortgage face; surplus may overshoot.
        let mut i = 0;
        while i < self.assets.len() {
            if self.cash >= target {
                break;
            }
            if let Asset::Property {
                sqft,
                price_per_sqft,
                mortgage,
                ..
            } = &mut self.assets[i]
            {
                let face = match &mortgage.item {
                    CashflowItem::Mortgage {
                        monthly,
                        period,
                        paid,
                    } => monthly * (*period as f64 - *paid as f64),
                    _ => 0.0,
                };
                self.cash += *sqft * *price_per_sqft - face;
                self.assets.remove(i);
                continue;
            }
            i += 1;
        }
    }
    fn run(&mut self, n: u16) -> Vec<Stats> {
        let mut stats = Vec::new();
        for _ in 0..=n {
            if self.terminated {
                break;
            }
            stats.push(self.once())
        }
        stats
    }
}

#[derive(Deserialize, Debug)]
struct ScenarioInput {
    cash: f64,
    #[serde(default)]
    reserve_months: u32,
    #[serde(default)]
    tax: Option<TaxConfig>,
    #[serde(default)]
    start: Option<String>,
    #[serde(default)]
    end: Option<String>,
    #[serde(default)]
    saving: Option<f64>,
    #[serde(default)]
    goals: Vec<Goal>,
    events: Vec<Event>,
}

impl ScenarioInput {
    fn horizon(&self) -> Result<u16, String> {
        match (&self.start, &self.end) {
            (Some(s), Some(e)) => {
                let s_ym = parse_ym(s).map_err(|m| format!("start: {m}"))?;
                let e_ym = parse_ym(e).map_err(|m| format!("end: {m}"))?;
                let months = months_between(s_ym, e_ym)? + 1;
                u16::try_from(months).map_err(|_| format!("horizon {months} exceeds u16"))
            }
            (None, None) => Ok(360),
            _ => Err("start and end must both be present or both absent".to_string()),
        }
    }
    fn into_scenario(self) -> Result<Scenario, String> {
        if let Some(s) = self.saving {
            if s < 0.0 {
                return Err(format!("saving must be non-negative, got {s}"));
            }
        }
        let allowance_override = self
            .tax
            .as_ref()
            .filter(|t| t.is_non_trivial())
            .map(|t| t.annual_basic());
        let horizon = self.horizon()?;
        // Goal validation: empty name, non-positive target, by_month > horizon,
        // duplicates (same name + by_month). Errors are surfaced from `load`
        // with the file path prefix.
        let mut seen = std::collections::HashSet::new();
        for (i, g) in self.goals.iter().enumerate() {
            if g.name.is_empty() {
                return Err(format!("goals[{i}]: name must not be empty"));
            }
            if g.target <= 0.0 {
                return Err(format!(
                    "goals[{i}] {:?}: target must be positive, got {}",
                    g.name, g.target
                ));
            }
            if g.by_month > horizon {
                return Err(format!(
                    "goals[{i}] {:?}: by_month {} exceeds horizon {}",
                    g.name, g.by_month, horizon
                ));
            }
            if !seen.insert((g.name.as_str(), g.by_month)) {
                return Err(format!(
                    "duplicate goal: name {:?}, by_month {}",
                    g.name, g.by_month
                ));
            }
        }
        Ok(Scenario {
            at: 0,
            cash: self.cash,
            cashflow: Vec::new(),
            assets: Vec::new(),
            events: self.events,
            reserve_months: self.reserve_months,
            salary_accrued: 0.0,
            mpf_accrued: 0.0,
            rent_accrued: 0.0,
            allowance_override,
            saving_target: self.saving,
            saving_breach_month: None,
            min_cash: self.cash,
            horizon,
            goals: self.goals,
            terminated: false,
        })
    }
}

fn load(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let input: ScenarioInput =
        serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let path_str = path.display().to_string();
    input
        .into_scenario()
        .map_err(|e| format!("{path_str}: {e}"))
}

/// CLI mode: a single scenario path or a `compare` subcommand with N
/// scenario paths. `--json` is captured separately and applies to either.
#[derive(Debug, PartialEq)]
enum Mode {
    Single(String),
    Compare(Vec<String>),
}

/// Parse CLI args: `--json` may appear anywhere; the first non-flag arg
/// decides `Mode` (`compare` → `Compare`, anything else → `Single` with
/// that arg as the path, defaulting to `scenario.yaml` when no non-flag
/// arg is given); any other `-`-prefixed arg is an error.
fn parse_args(args: &[String]) -> Result<(Mode, bool), String> {
    let mut positional: Vec<String> = Vec::new();
    let mut json = false;
    for a in args {
        if a == "--json" {
            json = true;
        } else if a.starts_with('-') {
            return Err(format!("unknown flag: {a}"));
        } else {
            positional.push(a.clone());
        }
    }
    match positional.first().map(String::as_str) {
        None => Ok((Mode::Single("scenario.yaml".to_string()), json)),
        Some("compare") => Ok((Mode::Compare(positional[1..].to_vec()), json)),
        Some(path) => Ok((Mode::Single(path.to_string()), json)),
    }
}

/// Evaluate each goal against the post-run `stats` series. The evaluation
/// month is the smaller of the goal's `by_month` and the terminal month
/// (when a `death` event from `add-insurance-modeling` ended the run early);
/// without a death event the terminal month is `None` and the goal evaluates
/// at its own `by_month`. Within a single `by_month`, the order returned
/// matches the YAML order (`Vec::sort_by_key` is stable). Outcomes carry the
/// user's `by_month` for JSON output and the actual `evaluation_month` for
/// the text summary.
fn evaluate_goals(
    stats: &[Stats],
    goals: &[Goal],
    terminal_month: Option<u16>,
) -> Vec<GoalOutcome> {
    if goals.is_empty() {
        return Vec::new();
    }
    let horizon = stats.len().saturating_sub(1) as u16;
    let mut outcomes: Vec<GoalOutcome> = goals
        .iter()
        .map(|g| {
            let eval_target = terminal_month.map_or(g.by_month, |t| g.by_month.min(t));
            let eval_month = eval_target.min(horizon);
            let stat = &stats[eval_month as usize];
            let value = match g.kind {
                GoalKind::Cash => stat.cash,
                GoalKind::NetWorth => stat.cash + stat.assets_value,
            };
            GoalOutcome {
                name: g.name.clone(),
                target: g.target,
                by_month: g.by_month,
                kind: g.kind,
                met: value >= g.target,
                value,
                evaluation_month: eval_month,
            }
        })
        .collect();
    outcomes.sort_by_key(|o| o.by_month);
    outcomes
}

/// Serialize the run as a JSON wrapper object. `months` carries the per-month
/// series (one object per simulated month with `month`, `cash`,
/// `assets_value`, `monthly_cashflow`), `goals` carries one entry per
/// declared goal with `name`, `target`, `by_month`, `kind`, `met`, `value`.
/// `terminal` is `true` when a `death` event ended the run early, and
/// `terminal_month` is the integer month index at which the run stopped;
/// both fields are present and `terminal_month` is set only when `terminal`
/// is `true`.
fn stats_json(stats: &[Stats], goals: &[GoalOutcome], terminal_month: Option<u16>) -> String {
    #[derive(Serialize)]
    struct Row<'a> {
        month: usize,
        #[serde(flatten)]
        stats: &'a Stats,
    }
    #[derive(Serialize)]
    struct Wrapper<'a> {
        terminal: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_month: Option<u16>,
        months: Vec<Row<'a>>,
        goals: &'a [GoalOutcome],
    }
    let rows: Vec<Row> = stats.iter().enumerate().map(|(month, s)| Row { month, stats: s }).collect();
    serde_json::to_string(&Wrapper {
        terminal: terminal_month.is_some(),
        terminal_month,
        months: rows,
        goals,
    })
    .expect("stats JSON never fails")
}

/// Derived run-level statistics shown by the TUI header. Insolvency uses the
/// same `cash < 0.0` rule as the text summary; the minimum is reported with
/// the first month it occurs at (ties go to the earliest month).
#[derive(Debug)]
struct KeyStats {
    final_cash: f64,
    final_assets_value: f64,
    final_monthly_cashflow: f64,
    min_cash: f64,
    min_cash_month: usize,
    first_insolvent_month: Option<usize>,
}

fn key_stats(stats: &[Stats]) -> KeyStats {
    if stats.is_empty() {
        return KeyStats {
            final_cash: 0.0,
            final_assets_value: 0.0,
            final_monthly_cashflow: 0.0,
            min_cash: 0.0,
            min_cash_month: 0,
            first_insolvent_month: None,
        };
    }
    let last = stats.last().unwrap();
    let mut min_cash = stats[0].cash;
    let mut min_cash_month = 0;
    let mut first_insolvent = None;
    for (i, s) in stats.iter().enumerate() {
        if s.cash < min_cash {
            min_cash = s.cash;
            min_cash_month = i;
        }
        if first_insolvent.is_none() && s.cash < 0.0 {
            first_insolvent = Some(i);
        }
    }
    KeyStats {
        final_cash: last.cash,
        final_assets_value: last.assets_value,
        final_monthly_cashflow: last.monthly_cashflow,
        min_cash,
        min_cash_month,
        first_insolvent_month: first_insolvent,
    }
}

/// Format the non-terminal human summary byte-for-byte the way `main` always
/// has: the final-month line, then the insolvent line when one exists,
/// then the saving-target breach line when one is configured, then one line
/// per declared goal (already ordered by `by_month` ascending by the
/// evaluator) in `goal <name> met|missed (...)` form. Empty goals leaves the
/// output byte-identical to the pre-goal version. When `terminal_month` is
/// `Some(N)`, the run was cut short by a `death` event and a final
/// `terminal at month N (death event)` line is appended after the goals.
fn format_text_summary(
    stats: &[Stats],
    saving_breach: Option<u16>,
    goals: &[GoalOutcome],
    terminal_month: Option<u16>,
) -> String {
    let last = stats.last().unwrap();
    let months = stats.len().saturating_sub(1);
    let mut out = format!(
        "after {months} months: cash {:.2}, assets {:.2}, monthly cashflow {:.2}\n",
        last.cash, last.assets_value, last.monthly_cashflow
    );
    if let Some((month, _)) = stats.iter().enumerate().find(|(_, s)| s.cash < 0.0) {
        out.push_str(&format!("insolvent from month {month}\n"));
    }
    if let Some(month) = saving_breach {
        out.push_str(&format!("saving target breached at month {month}\n"));
    }
    for g in goals {
        if g.met {
            out.push_str(&format!(
                "goal {} met ({:.2} at month {})\n",
                g.name, g.value, g.evaluation_month
            ));
        } else {
            out.push_str(&format!(
                "goal {} missed ({:.2} at month {}, target {:.2})\n",
                g.name, g.value, g.evaluation_month, g.target
            ));
        }
    }
    if let Some(month) = terminal_month {
        out.push_str(&format!("terminal at month {month} (death event)\n"));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, json) = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    match mode {
        Mode::Compare(paths) => {
            if paths.is_empty() {
                eprintln!("usage: ender compare <path>...");
                std::process::exit(1);
            }
            let paths: Vec<std::path::PathBuf> =
                paths.into_iter().map(std::path::PathBuf::from).collect();
            if let Err(e) = compare::run(&paths, json) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Mode::Single(path) => {
            let mut scenario = match load(std::path::Path::new(&path)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };
            let horizon = scenario.horizon;
            let stats = scenario.run(horizon);
            let terminal_month = scenario.terminated.then(|| stats.len() as u16 - 1);
            let goals = evaluate_goals(&stats, &scenario.goals, terminal_month);
            if json {
                println!("{}", stats_json(&stats, &goals, terminal_month));
                return;
            }
            if std::io::stdout().is_terminal() {
                let keys = key_stats(&stats);
                if let Err(e) = tui::run(&stats, &keys, &goals, terminal_month) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                return;
            }
            print!(
                "{}",
                format_text_summary(&stats, scenario.saving_breach_month, &goals, terminal_month)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_home_deducts_full_price_from_cash() {
        let yaml = "
cash: 50000
events:
  - when: 0
    type: buy_home
    property: { sqft: 1000, price_per_sqft: 30, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 12 } } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, 20000.0); // 50000 - 1000*30
        assert_eq!(stats[0].monthly_cashflow, 0.0); // one-time cost, not a flow
        assert_eq!(stats[0].assets_value, 30000.0);
    }

    #[test]
    fn investment_deducts_principal_from_cash() {
        let yaml = "
cash: 20000
events:
  - when: 0
    type: investment
    fund: { principal: 5000, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, 15000.0);
        assert_eq!(stats[0].assets_value, 5000.0); // principal starts intact
    }

    #[test]
    fn purchase_price_deducted_exactly_once() {
        let yaml = "
cash: 50000
events:
  - when: 0
    type: buy_home
    property: { sqft: 1000, price_per_sqft: 30, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 100, period: 240 } } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(2);
        assert_eq!(stats[0].cash, 19900.0); // 50000 - 30000 price - 100 mortgage
        assert_eq!(stats[1].cash - stats[0].cash, -100.0);
        assert_eq!(stats[2].cash - stats[1].cash, -100.0);
    }

    #[test]
    fn mortgage_period_allows_u16_range() {
        // 300 = 25-year loan; u8 saturated this to 255 (see scenarios/NOTES.md)
        let yaml = "
cash: 0
events:
  - when: 0
    type: buy_home
    property: { sqft: 1, price_per_sqft: 0, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 100, period: 300 } } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(1);
        assert_eq!(stats[0].cash, -100.0);
    }

    #[test]
    fn layoff_with_id_removes_that_salary() {
        let yaml = "
cash: 0
events:
  - { when: 0, type: job, id: me, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 0, type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.0 } }
  - { when: 2, type: layoff, id: partner }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(3);
        assert_eq!(stats[1].monthly_cashflow, 13300.0); // net of MPF
        assert_eq!(stats[2].monthly_cashflow, 7600.0);
        assert_eq!(stats[3].monthly_cashflow, 7600.0);
    }

    #[test]
    fn layoff_without_id_removes_first_salary() {
        let yaml = "
cash: 0
events:
  - { when: 0, type: job, id: me, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 0, type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.0 } }
  - { when: 1, type: layoff }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(2);
        assert_eq!(stats[1].monthly_cashflow, 5700.0);
        assert_eq!(stats[2].monthly_cashflow, 5700.0);
    }

    #[test]
    fn graduate_with_id_removes_that_tuition() {
        let yaml = "
cash: 0
events:
  - { when: 0, type: expense, id: kid-1, tuition: { monthly: 400, annualized_rate: 0.0 } }
  - { when: 0, type: expense, id: kid-2, tuition: { monthly: 600, annualized_rate: 0.0 } }
  - { when: 1, type: graduate, id: kid-2 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(2);
        assert_eq!(stats[0].monthly_cashflow, -1000.0);
        assert_eq!(stats[1].monthly_cashflow, -400.0);
        assert_eq!(stats[2].monthly_cashflow, -400.0);
    }

    #[test]
    fn layoff_with_unknown_id_is_noop() {
        let yaml = "
cash: 100
events:
  - { when: 0, type: job, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 1, type: layoff, id: ghost }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(2);
        assert_eq!(stats[1].cash, 15300.0); // no-op: both months paid, net of MPF
        assert_eq!(stats[2].monthly_cashflow, 7600.0);
    }

    #[test]
    fn tuition_event_loads_and_adds_tuition() {
        let yaml = "
cash: 0
events:
  - when: 0
    type: tuition
    tuition: { monthly: 800, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        s.once();
        assert_eq!(s.cashflow.len(), 1);
        assert!(matches!(s.cashflow[0].item, CashflowItem::Tuition { .. }));
    }

    #[test]
    fn inition_is_rejected() {
        let path = write_tmp(
            "ender_inition.yaml",
            "cash: 0\nevents:\n  - { when: 0, type: inition, tuition: { monthly: 800, annualized_rate: 0.0 } }\n",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("inition"), "error: {err}");
    }

    #[test]
    fn expense_event_fires_exactly_once() {
        let mut s = Scenario {
            at: 0,
            cash: 0.0,
            cashflow: Vec::new(),
            assets: Vec::new(),
            events: vec![Event {
                when: 3,
                kind: EventType::Expense(Labeled {
                    id: None,
                    item: CashflowItem::Rent {
                        monthly: 2000.0,
                        annualized_rate: 0.0,
                    },
                }),
            }],
            reserve_months: 0,
            salary_accrued: 0.0,
            mpf_accrued: 0.0,
            rent_accrued: 0.0,
            allowance_override: None,
            saving_target: None,
            saving_breach_month: None,
            min_cash: 0.0,
            horizon: 12,
            goals: Vec::new(),
            terminated: false,
        };
        let stats = s.run(12);
        assert_eq!(stats.len(), 13);
        assert_eq!(stats[2].monthly_cashflow, 0.0);
        assert_eq!(stats[3].monthly_cashflow, -2000.0);
        assert_eq!(stats[11].monthly_cashflow, -2000.0);
    }

    #[test]
    fn labeled_salary_loads_with_id() {
        let yaml = "cash: 0\nevents:\n  - when: 0\n    type: job\n    id: partner\n    salary: { monthly: 6000, annualized_rate: 0.0 }";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        s.once();
        assert_eq!(s.cashflow.len(), 1);
        assert_eq!(s.cashflow[0].id.as_deref(), Some("partner"));
    }

    #[test]
    fn end_removes_specific_salary() {
        let yaml = "
cash: 0
events:
  - { when: 0, type: job, id: me, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 0, type: job, id: partner, salary: { monthly: 6000, annualized_rate: 0.0 } }
  - { when: 2, type: end, id: partner }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(4);
        assert_eq!(stats[1].monthly_cashflow, 13300.0); // net of MPF
        assert_eq!(stats[2].monthly_cashflow, 7600.0);
        assert_eq!(stats[3].monthly_cashflow, 7600.0);
    }

    #[test]
    fn end_stops_fund_contribution() {
        let yaml = "
cash: 0
events:
  - when: 0
    type: investment
    fund: { principal: 1000, annualized_rate: 0.0, investment: { id: 401k, investment: { monthly: 500 } } }
  - { when: 2, type: end, id: 401k }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(3);
        assert_eq!(stats[1].monthly_cashflow, -500.0);
        assert_eq!(stats[2].monthly_cashflow, 0.0);
        // principal keeps its past contributions, but no further ones land
        assert_eq!(stats[2].assets_value, 2000.0);
    }

    #[test]
    fn end_with_unknown_id_is_noop() {
        let yaml = "cash: 100\nevents:\n  - { when: 1, type: end, id: ghost }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(3);
        assert_eq!(stats[3].cash, 100.0);
        assert_eq!(stats[3].monthly_cashflow, 0.0);
    }

    #[test]
    fn one_off_expense_lands_once() {
        let yaml = "cash: 20000\nevents:\n  - { when: 3, type: one_off_expense, one_off: { amount: 5000 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(5);
        assert_eq!(stats[2].monthly_cashflow, 0.0);
        assert_eq!(stats[3].monthly_cashflow, -5000.0);
        assert_eq!(stats[3].cash, 15000.0);
        assert_eq!(stats[4].monthly_cashflow, 0.0);
        assert_eq!(stats[5].cash, 15000.0);
    }

    #[test]
    fn one_off_fired_defaults_to_false() {
        let yaml = "cash: 0\nevents:\n  - { when: 0, type: one_off_expense, one_off: { amount: 100 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(1);
        assert_eq!(stats[0].monthly_cashflow, -100.0);
        assert_eq!(stats[1].monthly_cashflow, 0.0);
    }

    #[test]
    fn one_off_income_lands_once() {
        let yaml = "cash: 0\nevents:\n  - { when: 6, type: one_off_income, one_off: { amount: 10000 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(8);
        assert_eq!(stats[5].monthly_cashflow, 0.0);
        assert_eq!(stats[6].monthly_cashflow, 10000.0);
        assert_eq!(stats[6].cash, 10000.0);
        assert_eq!(stats[7].monthly_cashflow, 0.0);
    }

    #[test]
    fn one_off_sign_comes_from_event_type() {
        let yaml = "cash: 0\nevents:\n  - { when: 0, type: one_off_expense, one_off: { amount: -5000 } }\n  - { when: 0, type: one_off_income, one_off: { amount: -1000 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].monthly_cashflow, -4000.0);
    }

    #[test]
    fn end_removes_a_one_off_before_it_fires() {
        let yaml = "cash: 20000\nevents:\n  - { when: 0, type: one_off_expense, id: tax, one_off: { amount: 5000 } }\n  - { when: 0, type: end, id: tax }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(1);
        assert_eq!(stats[0].monthly_cashflow, 0.0);
        assert_eq!(stats[1].cash, 20000.0);
    }

    fn write_tmp(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn minimal_scenario_loads() {
        let input: ScenarioInput = serde_yaml_ng::from_str("cash: 20000\nevents: []").unwrap();
        let s = input.into_scenario().unwrap();
        assert_eq!(s.cash, 20000.0);
        assert!(s.cashflow.is_empty());
        assert!(s.assets.is_empty());
        assert!(s.events.is_empty());
    }

    #[test]
    fn full_vocabulary_loads_and_fires() {
        let yaml = r#"
cash: 5000
events:
  - when: 0
    type: job
    id: main
    salary: { monthly: 8000, annualized_rate: 0.03 }
  - when: 0
    type: expense
    rent: { monthly: 2000, annualized_rate: 0.02 }
  - when: 6
    type: layoff
  - when: 0
    type: tuition
    tuition: { monthly: 800, annualized_rate: 0.02 }
  - { when: 8, type: graduate }
  - when: 9
    type: investment
    fund: { principal: 1000, annualized_rate: 0.05, investment: { id: 401k, investment: { monthly: 500 } } }
  - when: 12
    type: buy_home
    property: { sqft: 1200, price_per_sqft: 300, capex_per_sqft: 20, annualized_rate: 0.03, mortgage: { mortgage: { monthly: 1800, period: 240 } } }
  - when: 24
    type: buy_to_let
    property: { sqft: 900, price_per_sqft: 200, capex_per_sqft: 15, annualized_rate: 0.02, mortgage: { mortgage: { monthly: 1200, period: 120 } }, rental: { rental_income: { occupancy: 0.85, monthly: 2200, annualized_rate: 0.02 } } }
  - { when: 24, type: end, id: 401k }
  - { when: 2, type: one_off_expense, one_off: { amount: 5000 } }
  - { when: 3, type: one_off_income, one_off: { amount: 5000 } }
"#;
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(24);
        assert_eq!(stats.len(), 25);
        // salary fired then was removed by layoff; rent remains
        assert_eq!(s.cashflow.len(), 3);
        assert!(matches!(&s.cashflow[0].item, CashflowItem::Rent { monthly, .. } if *monthly > 2000.0));
        assert!(s.cashflow[1..].iter().all(|l| matches!(l.item, CashflowItem::OneOff { fired: true, .. })));
        // fund + home + rental property
        assert_eq!(s.assets.len(), 3);
        // the 401k contribution was ended at month 24
        match &s.assets[0] {
            Asset::Fund { investment, .. } => assert!(investment.is_none()),
            _ => panic!("expected fund as first asset"),
        }
    }

    #[test]
    fn mortgage_paid_defaults_to_zero() {
        let yaml = "cash: 0\nevents:\n  - when: 0\n    type: buy_home\n    property: { sqft: 1000, price_per_sqft: 100, capex_per_sqft: 10, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 1000, period: 12 } } }";
        let s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        match &s.events[0].kind {
            EventType::BuyHome(Asset::Property {
                mortgage:
                    Labeled {
                        item: CashflowItem::Mortgage { paid, period, .. },
                        ..
                    },
                ..
            }) => {
                assert_eq!(*paid, 0);
                assert_eq!(*period, 12);
            }
            _ => panic!("expected mortgaged property"),
        }
    }

    #[test]
    fn reserve_loads_from_yaml() {
        let yaml = "cash: 10000\nreserve_months: 3\nevents: []\n";
        let s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        assert_eq!(s.reserve_months, 3);
    }

    #[test]
    fn negative_reserve_fails_to_load() {
        let path = write_tmp("ender_neg_reserve.yaml", "cash: 0\nreserve_months: -1\nevents: []\n");
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_neg_reserve.yaml"), "error: {err}");
    }

    #[test]
    fn cash_above_target_is_untouched() {
        // reserve 3, cash 50000, rent 2000 -> target 6000, cash stays put
        let yaml = "
cash: 50000
reserve_months: 3
events:
  - when: 0
    type: expense
    rent: { monthly: 2000, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, 48000.0); // rent paid but no liquidation
        assert!(s.assets.is_empty());
    }

    #[test]
    fn absent_reserve_disables_settlement() {
        // No reserve_months: cash goes negative, fund sits untouched.
        let yaml = "
cash: 50
events:
  - when: 0
    type: expense
    rent: { monthly: 200, annualized_rate: 0.0 }
  - when: 0
    type: investment
    fund: { principal: 1000, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, -1150.0); // 50 - 1000 fund - 200 rent; no settlement
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 1000.0), // untouched
            _ => panic!("expected fund"),
        }
    }

    #[test]
    fn reserve_breached_only_when_assets_exhausted() {
        // reserve 3, post-deduction cash 50, rent 200, fund 100, no properties.
        // After rent: cash -150. Target 600, shortfall 750. Fund drawn to 0,
        // cash -50. No assets left, reserve breached. Run reports insolvent.
        let yaml = "
cash: 150
reserve_months: 3
events:
  - when: 0
    type: expense
    rent: { monthly: 200, annualized_rate: 0.0 }
  - when: 0
    type: investment
    fund: { principal: 100, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, -50.0);
        assert!(s.assets.is_empty());
    }

    #[test]
    fn property_sold_after_funds_exhausted() {
        // reserve 3, post-deduction cash 100, rent 200, fund 100, property
        // worth 30000 with mortgage 100/month over 12 periods (1100 remaining
        // face after this month's payment).
        // Event fire: cash = 30200 - 100 (fund) - 30000 (property) = 100.
        // Month flow: rent -200, mortgage -100 -> cashflow -300, cash -200.
        // Settle: target = 3 * (rent + mortgage) = 900; shortfall 1100.
        // Draw 100 from fund (principal 0), cash = -100. Sell property for 28900.
        // Cash 28800. Next month: rent only -200.
        let yaml = "
cash: 30200
reserve_months: 3
events:
  - when: 0
    type: expense
    rent: { monthly: 200, annualized_rate: 0.0 }
  - when: 0
    type: investment
    fund: { principal: 100, annualized_rate: 0.0 }
  - when: 0
    type: buy_home
    property: { sqft: 300, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 100, period: 12 } } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(1);
        assert_eq!(stats[0].cash, 28800.0);
        assert!(s.assets.is_empty());
        // next month: rent only -200 (property's mortgage flow is gone)
        assert_eq!(stats[1].monthly_cashflow, -200.0);
    }

    #[test]
    fn one_off_does_not_inflate_reserve_target() {
        // Spec: reserve 3, post-deduction cash 8000, rent 2000, fund 10000,
        // one_off_expense 5000 at month 0. Cash should end at 6000 (target = 3*2000
        // = 6000, shortfall 5000), proving the one-off is excluded from the base.
        // YAML cash = 18000 so the 10000 principal deduction leaves 8000.
        let yaml = "
cash: 18000
reserve_months: 3
events:
  - when: 0
    type: expense
    rent: { monthly: 2000, annualized_rate: 0.0 }
  - when: 0
    type: investment
    fund: { principal: 10000, annualized_rate: 0.0 }
  - when: 0
    type: one_off_expense
    one_off: { amount: 5000 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, 6000.0);
        assert_eq!(stats[0].monthly_cashflow, -7000.0); // rent -2000 + one-off -5000
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 5000.0), // drew 5000
            _ => panic!("expected fund"),
        }
    }

    #[test]
    fn fund_drawn_to_top_up_reserve() {
        // reserve 3, post-deduction cash 100, rent 200, fund 1000
        // YAML cash is 1100 so the principal deduction leaves 100 on hand.
        // After rent: cash -100; target = 3*200 = 600; shortfall 700; draw 700 from fund.
        // Cash ends 600, fund principal ends 300, monthly_cashflow -200.
        let yaml = "
cash: 1100
reserve_months: 3
events:
  - when: 0
    type: expense
    rent: { monthly: 200, annualized_rate: 0.0 }
  - when: 0
    type: investment
    fund: { principal: 1000, annualized_rate: 0.0 }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(0);
        assert_eq!(stats[0].cash, 600.0);
        assert_eq!(stats[0].monthly_cashflow, -200.0);
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 300.0),
            _ => panic!("expected fund"),
        }
    }

    #[test]
    fn unknown_event_type_fails_naming_file() {
        let path = write_tmp(
            "ender_unknown.yaml",
            "cash: 0\nevents:\n  - when: 0\n    type: lottery_win\n",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_unknown.yaml"), "error: {err}");
        assert!(err.contains("lottery_win"), "error: {err}");
    }

    #[test]
    fn malformed_yaml_fails_naming_file() {
        let path = write_tmp("ender_malformed.yaml", "cash: [unclosed");
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_malformed.yaml"), "error: {err}");
    }

    // ---- Hong Kong tax: pure math (IRD worked examples) ----

    #[test]
    fn mpf_below_cap_is_five_percent() {
        assert_eq!(mpf_monthly(20_000.0), 1_000.0);
    }

    #[test]
    fn mpf_capped_at_1500() {
        assert_eq!(mpf_monthly(40_000.0), 1_500.0);
    }

    #[test]
    fn salaries_tax_ird_progressive_example() {
        // gross 350k, MPF 18k, allowance 132k: chargeable 200k
        // progressive 16,000 vs standard 332k x 15% = 49,800
        assert_eq!(salaries_tax(350_000.0, 18_000.0, BASIC_ALLOWANCE), 16_000.0);
    }

    #[test]
    fn salaries_tax_zero_below_allowance() {
        assert_eq!(salaries_tax(100_000.0, 5_000.0, BASIC_ALLOWANCE), 0.0);
    }

    #[test]
    fn salaries_tax_two_tier_standard_rate_wins_high_income() {
        // net assessable income 7,982,000: 15% on first 5M + 16% on rest
        // = 1,227,120 (progressive would be 1,316,500)
        assert_eq!(
            salaries_tax(8_000_000.0, 18_000.0, BASIC_ALLOWANCE),
            1_227_120.0
        );
    }

    #[test]
    fn salaries_tax_allowance_override() {
        // 320k gross, 18k MPF, 145k allowance: chargeable 157k
        // -> 1,000 + 3,000 + 5,000 + 980 = 9,980
        assert_eq!(salaries_tax(320_000.0, 18_000.0, 145_000.0), 9_980.0);
    }

    #[test]
    fn property_tax_on_repair_allowed_rent() {
        assert_eq!(property_tax(300_000.0), 36_000.0);
        assert_eq!(property_tax(0.0), 0.0);
    }

    // ---- HK tax: integration ----

    fn scenario(yaml: &str) -> Scenario {
        serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap()
    }

    #[test]
    fn salary_cashflow_is_net_of_mpf() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 20000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(1);
        assert_eq!(stats[0].monthly_cashflow, 19_000.0);
        assert_eq!(stats[1].monthly_cashflow, 19_000.0);
    }

    #[test]
    fn salary_cashflow_mpf_capped() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 40000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(0);
        assert_eq!(stats[0].monthly_cashflow, 38_500.0);
    }

    #[test]
    fn salaries_tax_charged_at_block_end() {
        // 20k gross/mo: accrued 240k, MPF 12k, chargeable 96k -> tax 3,760
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 20000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(11);
        assert_eq!(stats[10].cash, 209_000.0); // 11 net months, no tax yet
        assert_eq!(stats[11].cash, 224_240.0); // 12 net months - 3,760
    }

    #[test]
    fn consecutive_blocks_taxed_identically() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 20000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(23);
        assert_eq!(stats[11].cash - stats[10].cash, 19_000.0 - 3_760.0);
        assert_eq!(stats[23].cash - stats[22].cash, 19_000.0 - 3_760.0);
        assert_eq!(stats[23].cash, 448_480.0);
    }

    #[test]
    fn partial_block_after_layoff_taxes_accrued_months_only() {
        // 25k gross for 6 of 12 months: accrued 150k, MPF 7.5k,
        // chargeable 10.5k -> tax 210 (full year would be 9,420)
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 25000, annualized_rate: 0.0 } }
  - { when: 6, type: layoff }",
        );
        let stats = s.run(11);
        assert_eq!(stats[11].cash, 6.0 * 23_750.0 - 210.0);
    }

    #[test]
    fn property_tax_uses_occupancy_adjusted_accrual() {
        // rent 10k at occupancy 0.5: accrued 60k, tax 7,200
        let mut s = scenario(
            "
cash: 10100
events:
  - when: 0
    type: buy_to_let
    property: { sqft: 100, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 1 } }, rental: { rental_income: { occupancy: 0.5, monthly: 10000, annualized_rate: 0.0 } } }",
        );
        let stats = s.run(11);
        assert_eq!(stats[11].cash, 100.0 + 12.0 * 5_000.0 - 7_200.0);
    }

    #[test]
    fn combined_salary_and_rent_bill_at_block_end() {
        // salaries tax 3,760 + property tax 36,000 in one deduction
        let mut s = scenario(
            "
cash: 10000
events:
  - { when: 0, type: job, salary: { monthly: 20000, annualized_rate: 0.0 } }
  - when: 0
    type: buy_to_let
    property: { sqft: 100, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 1 } }, rental: { rental_income: { occupancy: 1.0, monthly: 25000, annualized_rate: 0.0 } } }",
        );
        let stats = s.run(11);
        assert_eq!(stats[11].cash, 12.0 * 44_000.0 - 3_760.0 - 36_000.0);
    }

    #[test]
    fn tax_charge_precedes_reserve_settlement() {
        // reserve 3 x rent 20k = 60k target; net flow -1,000/mo topped up
        // from the fund. Month-11 tax 3,760 must join that month's draw.
        let mut s = scenario(
            "
cash: 80000
reserve_months: 3
events:
  - { when: 0, type: job, salary: { monthly: 20000, annualized_rate: 0.0 } }
  - { when: 0, type: expense, rent: { monthly: 20000, annualized_rate: 0.0 } }
  - { when: 0, type: investment, fund: { principal: 20000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(11);
        assert_eq!(stats[10].cash, 60_000.0);
        assert_eq!(stats[10].assets_value, 9_000.0); // 11 top-ups of 1,000
        assert_eq!(stats[11].cash, 60_000.0); // tax covered by the draw
        assert_eq!(stats[11].assets_value, 4_240.0); // 9,000 - 1,000 - 3,760
    }

    // ---- CLI argument parsing (TODO 6) ----

    #[test]
    fn parse_args_defaults_to_scenario_yaml() {
        let (mode, json) = parse_args(&[]).unwrap();
        assert_eq!(mode, Mode::Single("scenario.yaml".to_string()));
        assert!(!json);
    }

    #[test]
    fn parse_args_takes_explicit_path() {
        let (mode, json) = parse_args(&["s.yaml".to_string()]).unwrap();
        assert_eq!(mode, Mode::Single("s.yaml".to_string()));
        assert!(!json);
    }

    #[test]
    fn parse_args_recognizes_json_alone() {
        let (mode, json) = parse_args(&["--json".to_string()]).unwrap();
        assert_eq!(mode, Mode::Single("scenario.yaml".to_string()));
        assert!(json);
    }

    #[test]
    fn parse_args_recognizes_json_before_path() {
        let (mode, json) = parse_args(&["--json".to_string(), "s.yaml".to_string()]).unwrap();
        assert_eq!(mode, Mode::Single("s.yaml".to_string()));
        assert!(json);
    }

    #[test]
    fn parse_args_recognizes_json_after_path() {
        let (mode, json) = parse_args(&["s.yaml".to_string(), "--json".to_string()]).unwrap();
        assert_eq!(mode, Mode::Single("s.yaml".to_string()));
        assert!(json);
    }

    #[test]
    fn parse_args_rejects_unknown_flag_naming_it() {
        let err = parse_args(&["--yaml".to_string()]).unwrap_err();
        assert!(err.contains("--yaml"), "error should name the flag: {err}");
    }

    #[test]
    fn parse_args_routes_compare_with_two_paths() {
        let (mode, json) =
            parse_args(&["compare".to_string(), "a.yaml".to_string(), "b.yaml".to_string()])
                .unwrap();
        assert_eq!(
            mode,
            Mode::Compare(vec!["a.yaml".to_string(), "b.yaml".to_string()])
        );
        assert!(!json);
    }

    #[test]
    fn parse_args_compare_with_no_paths() {
        let (mode, _) = parse_args(&["compare".to_string()]).unwrap();
        assert_eq!(mode, Mode::Compare(vec![]));
    }

    #[test]
    fn parse_args_compare_with_one_path() {
        let (mode, _) = parse_args(&["compare".to_string(), "s.yaml".to_string()]).unwrap();
        assert_eq!(mode, Mode::Compare(vec!["s.yaml".to_string()]));
    }

    #[test]
    fn parse_args_json_before_compare() {
        let (mode, json) = parse_args(
            &["--json".to_string(), "compare".to_string(), "a.yaml".to_string()],
        )
        .unwrap();
        assert_eq!(mode, Mode::Compare(vec!["a.yaml".to_string()]));
        assert!(json);
    }

    #[test]
    fn parse_args_json_after_compare_paths() {
        let (mode, json) = parse_args(
            &["compare".to_string(), "a.yaml".to_string(), "--json".to_string()],
        )
        .unwrap();
        assert_eq!(mode, Mode::Compare(vec!["a.yaml".to_string()]));
        assert!(json);
    }

    #[test]
    fn parse_args_unknown_flag_after_compare() {
        let err = parse_args(
            &["compare".to_string(), "a.yaml".to_string(), "--yaml".to_string()],
        )
        .unwrap_err();
        assert!(err.contains("--yaml"), "error should name the flag: {err}");
    }

    // ---- Key statistics helper (TODO 7) ----

    fn s(cash: f64, assets_value: f64, cashflow: f64) -> Stats {
        Stats {
            cash,
            assets_value,
            monthly_cashflow: cashflow,
        }
    }

    #[test]
    fn key_stats_returns_final_values() {
        let stats = vec![s(100.0, 50.0, 25.0), s(75.0, 60.0, -25.0)];
        let k = key_stats(&stats);
        assert_eq!(k.final_cash, 75.0);
        assert_eq!(k.final_assets_value, 60.0);
        assert_eq!(k.final_monthly_cashflow, -25.0);
    }

    #[test]
    fn key_stats_finds_min_cash_with_month() {
        let stats = vec![s(100.0, 0.0, 0.0), s(50.0, 0.0, 0.0), s(-200.0, 0.0, 0.0), s(80.0, 0.0, 0.0)];
        let k = key_stats(&stats);
        assert_eq!(k.min_cash, -200.0);
        assert_eq!(k.min_cash_month, 2);
    }

    #[test]
    fn key_stats_min_cash_ties_pick_first_month() {
        let stats = vec![s(50.0, 0.0, 0.0), s(100.0, 0.0, 0.0), s(50.0, 0.0, 0.0)];
        let k = key_stats(&stats);
        assert_eq!(k.min_cash, 50.0);
        assert_eq!(k.min_cash_month, 0);
    }

    #[test]
    fn key_stats_finds_first_insolvent_month() {
        let stats = vec![s(100.0, 0.0, 0.0), s(-1.0, 0.0, 0.0), s(-50.0, 0.0, 0.0)];
        let k = key_stats(&stats);
        assert_eq!(k.first_insolvent_month, Some(1));
    }

    #[test]
    fn key_stats_no_insolvent_is_none() {
        let stats = vec![s(100.0, 0.0, 0.0), s(50.0, 0.0, 0.0), s(0.0, 0.0, 0.0)];
        let k = key_stats(&stats);
        assert_eq!(k.first_insolvent_month, None);
    }

    #[test]
    fn key_stats_empty_series() {
        let k = key_stats(&[]);
        assert_eq!(k.final_cash, 0.0);
        assert_eq!(k.final_assets_value, 0.0);
        assert_eq!(k.final_monthly_cashflow, 0.0);
        assert_eq!(k.min_cash, 0.0);
        assert_eq!(k.min_cash_month, 0);
        assert_eq!(k.first_insolvent_month, None);
    }

    #[test]
    fn format_text_summary_pipes_byte_for_byte() {
        // The non-terminal text path must reproduce the pre-TUI summary byte-for-byte.
        let stats = vec![s(100.0, 50.0, 25.0), s(75.0, 60.0, -25.0)];
        let out = format_text_summary(&stats, None, &[], None);
        assert_eq!(
            out,
            "after 1 months: cash 75.00, assets 60.00, monthly cashflow -25.00\n"
        );
    }

    #[test]
    fn format_text_summary_appends_insolvent_line() {
        let stats = vec![s(100.0, 0.0, 0.0), s(-1.0, 0.0, 0.0), s(50.0, 0.0, 0.0)];
        let out = format_text_summary(&stats, None, &[], None);
        assert_eq!(
            out,
            "after 2 months: cash 50.00, assets 0.00, monthly cashflow 0.00\n\
             insolvent from month 1\n"
        );
    }

    #[test]
    fn format_text_summary_omits_insolvent_line_when_solvent() {
        let stats = vec![s(100.0, 0.0, 0.0), s(50.0, 0.0, 0.0), s(0.5, 0.0, 0.0)];
        let out = format_text_summary(&stats, None, &[], None);
        assert_eq!(
            out,
            "after 2 months: cash 0.50, assets 0.00, monthly cashflow 0.00\n"
        );
    }

    // ---- JSON series output (TODO 6) ----

    #[test]
    fn stats_json_emits_one_object_per_month_with_index() {
        let yaml = "
cash: 20000
events:
  - { when: 0, type: job, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 0, type: expense, rent: { monthly: 2000, annualized_rate: 0.0 } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(12);
        let parsed: serde_json::Value = serde_json::from_str(&stats_json(&stats, &[], None)).unwrap();
        let months = parsed["months"].as_array().unwrap();
        assert_eq!(months.len(), 13);
        for (i, obj) in months.iter().enumerate() {
            assert_eq!(obj["month"], i);
            assert!(obj["cash"].is_number());
            assert!(obj["assets_value"].is_number());
            assert!(obj["monthly_cashflow"].is_number());
        }
    }

    #[test]
    fn stats_json_final_element_matches_summary_line() {
        let yaml = "
cash: 20000
events:
  - { when: 0, type: job, salary: { monthly: 8000, annualized_rate: 0.0 } }
  - { when: 0, type: expense, rent: { monthly: 2000, annualized_rate: 0.0 } }
";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario().unwrap();
        let stats = s.run(12);
        let parsed: serde_json::Value = serde_json::from_str(&stats_json(&stats, &[], None)).unwrap();
        let months = parsed["months"].as_array().unwrap();
        let last = months.last().unwrap();
        let s_last = stats.last().unwrap();
        assert_eq!(last["cash"].as_f64().unwrap(), s_last.cash);
        assert_eq!(last["assets_value"].as_f64().unwrap(), s_last.assets_value);
        assert_eq!(
            last["monthly_cashflow"].as_f64().unwrap(),
            s_last.monthly_cashflow
        );
    }

    // ---- TaxConfig::annual_basic (IRD 2026/27 household allowances) ----

    fn yaml_to_tax(yaml: &str) -> TaxConfig {
        serde_yaml_ng::from_str::<TaxConfig>(yaml).unwrap()
    }

    #[test]
    fn annual_basic_married_two_kids_two_noncohabiting_parents_60_plus() {
        // 290k married + 2*140k children + 2*55k parents = 680,000
        let t = yaml_to_tax(
            "status: married
children: 2
parents:
  - age_band: '60+'
    living_with: false
  - age_band: '60+'
    living_with: false",
        );
        assert_eq!(t.annual_basic(), 680_000.0);
    }

    #[test]
    fn annual_basic_single_one_cohabiting_parent_55_to_59() {
        // 145k single + 55k parent (27.5k doubled to 55k because living_with) = 200,000
        let t = yaml_to_tax(
            "status: single
parents:
  - age_band: '55-59'
    living_with: true",
        );
        assert_eq!(t.annual_basic(), 200_000.0);
    }

    #[test]
    fn annual_basic_empty_block_is_not_non_trivial() {
        let t = yaml_to_tax("{}");
        assert!(!t.is_non_trivial());
        assert_eq!(t.annual_basic(), SINGLE_BASIC);
    }

    #[test]
    fn annual_basic_unknown_field_fails_to_load() {
        let err = serde_yaml_ng::from_str::<TaxConfig>("shoe_size: 42")
            .unwrap_err()
            .to_string();
        assert!(err.contains("shoe_size"), "error should name the field: {err}");
    }

    #[test]
    fn annual_basic_parent_missing_age_band_fails_to_load() {
        let err = serde_yaml_ng::from_str::<TaxConfig>("parents: [{ living_with: false }]")
            .unwrap_err()
            .to_string();
        assert!(err.contains("age_band"), "error: {err}");
    }

    #[test]
    fn married_tax_block_pays_tax_against_derived_total() {
        // Tax against 680k allowance yields less than against 132k default.
        // 240k * 12mo gross = 2,880,000. MPF 18,000. Chargeable against
        // 680k allowance = 2,182,000 -> progressive 320,540 vs standard
        // 431,700 -> min = 320,540. Against 132k default = 2,730,000 ->
        // progressive 432,100 vs standard 431,700 -> min = 431,700. Diff =
        // 111,160.
        let mut s = scenario(
            "cash: 0
tax:
  status: married
  children: 2
  parents:
    - age_band: '60+'
      living_with: false
    - age_band: '60+'
      living_with: false
events:
  - { when: 0, type: job, salary: { monthly: 240000, annualized_rate: 0.0 } }",
        );
        let stats = s.run(11);
        // expected cash: 12 * (240000 - 1500) - salaries_tax(2_880_000, 18_000, 680_000) - 0
        let net = (240_000.0 - 1_500.0) * 12.0;
        let tax = salaries_tax(2_880_000.0, 18_000.0, 680_000.0);
        assert!((stats[11].cash - (net - tax)).abs() < 0.01, "got {}, expected {}", stats[11].cash, net - tax);
    }

    // ---- Date parsing ----

    #[test]
    fn parse_ym_accepts_valid() {
        assert_eq!(parse_ym("2026-08").unwrap(), (2026, 8));
    }

    #[test]
    fn parse_ym_rejects_garbage() {
        assert!(parse_ym("nope").is_err());
        assert!(parse_ym("2026/08").is_err());
        assert!(parse_ym("2026-13").is_err());
        assert!(parse_ym("2026-00").is_err());
    }

    #[test]
    fn months_between_inclusive() {
        let s = (2026, 8);
        assert_eq!(months_between(s, (2026, 8)).unwrap(), 0); // same -> 0
        assert_eq!(months_between(s, (2026, 9)).unwrap(), 1);
        assert_eq!(months_between(s, (2029, 8)).unwrap(), 36);
    }

    #[test]
    fn months_between_rejects_reversed() {
        assert!(months_between((2026, 8), (2026, 7)).is_err());
    }

    // ---- Scenario: start / end / saving ----

    #[test]
    fn end_truncates_run_to_horizon() {
        let yaml = "
cash: 0
start: 2026-08
end: 2029-08
events: []
";
        let s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario()
            .unwrap();
        assert_eq!(s.horizon, 37); // Aug 2026 to Aug 2029 inclusive = 37 months
    }

    #[test]
    fn absent_end_uses_360_default() {
        let s = scenario("cash: 0\nevents: []");
        assert_eq!(s.horizon, 360);
    }

    #[test]
    fn only_one_of_start_end_fails_to_load() {
        assert!(serde_yaml_ng::from_str::<ScenarioInput>("cash: 0\nstart: 2026-08\nevents: []")
            .unwrap()
            .into_scenario()
            .is_err());
    }

    #[test]
    fn negative_saving_fails_to_load() {
        assert!(serde_yaml_ng::from_str::<ScenarioInput>("cash: 0\nsaving: -1\nevents: []")
            .unwrap()
            .into_scenario()
            .is_err());
    }

    #[test]
    fn saving_breach_records_first_month() {
        let mut s = scenario(
            "cash: 100
saving: 200
events:
  - { when: 0, type: expense, rent: { monthly: 50, annualized_rate: 0.0 } }",
        );
        s.run(2);
        assert_eq!(s.saving_breach_month, Some(0));
    }

    #[test]
    fn saving_unbreached_records_none() {
        let mut s = scenario(
            "cash: 10000
saving: 100
events:
  - { when: 0, type: expense, rent: { monthly: 1, annualized_rate: 0.0 } }",
        );
        s.run(2);
        assert_eq!(s.saving_breach_month, None);
    }

    #[test]
    fn format_text_summary_appends_saving_breach_line() {
        let stats = vec![s(100.0, 0.0, 0.0), s(50.0, 0.0, 0.0)];
        let out = format_text_summary(&stats, Some(0), &[], None);
        assert!(out.contains("saving target breached at month 0\n"), "got: {out}");
    }

    // ---- Goal tracking ----

    fn goal(name: &str, target: f64, by_month: u16, kind: GoalKind) -> Goal {
        Goal {
            name: name.to_string(),
            target,
            by_month,
            kind,
        }
    }

    #[test]
    fn goal_cash_met() {
        let stats = vec![
            s(100.0, 0.0, 0.0),
            s(200_000.0, 0.0, 0.0),
            s(250_000.0, 0.0, 0.0),
        ];
        let g = goal("college", 200_000.0, 2, GoalKind::Cash);
        let outcomes = evaluate_goals(&stats, &[g], None);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].name, "college");
        assert!(outcomes[0].met);
        assert_eq!(outcomes[0].value, 250_000.0);
        assert_eq!(outcomes[0].by_month, 2);
        assert_eq!(outcomes[0].evaluation_month, 2);
    }

    #[test]
    fn goal_cash_missed() {
        let stats = vec![s(100.0, 0.0, 0.0), s(200_000.0, 0.0, 0.0), s(150_000.0, 0.0, 0.0)];
        let g = goal("college", 200_000.0, 2, GoalKind::Cash);
        let outcomes = evaluate_goals(&stats, &[g], None);
        assert_eq!(outcomes.len(), 1);
        assert!(!outcomes[0].met);
        assert_eq!(outcomes[0].value, 150_000.0);
        assert_eq!(outcomes[0].by_month, 2);
        assert_eq!(outcomes[0].target, 200_000.0);
    }

    #[test]
    fn goal_net_worth_uses_assets() {
        let stats = vec![
            s(50_000.0, 1_100_000.0, 0.0),
        ];
        let g = goal("retirement", 1_000_000.0, 0, GoalKind::NetWorth);
        let outcomes = evaluate_goals(&stats, &[g], None);
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].met);
        assert_eq!(outcomes[0].value, 1_150_000.0);
    }

    #[test]
    fn goal_net_worth_missed() {
        let stats = vec![s(50_000.0, 800_000.0, 0.0)];
        let g = goal("retirement", 1_000_000.0, 0, GoalKind::NetWorth);
        let outcomes = evaluate_goals(&stats, &[g], None);
        assert!(!outcomes[0].met);
        assert_eq!(outcomes[0].value, 850_000.0);
        assert_eq!(outcomes[0].target, 1_000_000.0);
    }

    #[test]
    fn goal_default_kind_is_cash() {
        let yaml = "
cash: 250000
goals:
  - { name: college, target: 200000, by_month: 0 }
events: []
";
        let s = serde_yaml_ng::from_str::<ScenarioInput>(yaml).unwrap().into_scenario().unwrap();
        assert_eq!(s.goals.len(), 1);
        assert!(matches!(s.goals[0].kind, GoalKind::Cash));
    }

    #[test]
    fn goals_evaluated_in_by_month_order() {
        let stats = vec![
            s(100.0, 0.0, 0.0),
            s(300_000.0, 0.0, 0.0), // month 1 — met
            s(200_000.0, 0.0, 0.0), // month 2 — at target
            s(1_200_000.0, 0.0, 0.0), // month 3 — met (retirement)
        ];
        let goals = vec![
            goal("retirement", 1_000_000.0, 3, GoalKind::Cash),
            goal("college", 250_000.0, 1, GoalKind::Cash),
        ];
        let outcomes = evaluate_goals(&stats, &goals, None);
        // college (by_month 1) first, retirement (by_month 3) second
        assert_eq!(outcomes[0].name, "college");
        assert_eq!(outcomes[1].name, "retirement");
        assert!(outcomes[0].met);
        assert!(outcomes[1].met);
    }

    #[test]
    fn goal_past_horizon_fails_to_load() {
        let path = write_tmp(
            "ender_goal_past_horizon.yaml",
            "cash: 0
start: 2026-01
end: 2027-01
goals:
  - { name: college, target: 200000, by_month: 500 }
events: []
",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_goal_past_horizon.yaml"), "error: {err}");
        assert!(err.contains("exceeds horizon"), "error: {err}");
    }

    #[test]
    fn duplicate_goal_fails_to_load() {
        let path = write_tmp(
            "ender_dup_goal.yaml",
            "cash: 0
goals:
  - { name: college, target: 200000, by_month: 12 }
  - { name: college, target: 200000, by_month: 12 }
events: []
",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_dup_goal.yaml"), "error: {err}");
        assert!(err.contains("duplicate goal"), "error: {err}");
        assert!(err.contains("college"), "error: {err}");
    }

    #[test]
    fn empty_goal_name_fails_to_load() {
        let path = write_tmp(
            "ender_empty_goal_name.yaml",
            "cash: 0
goals:
  - { name: \"\", target: 1000, by_month: 0 }
events: []
",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_empty_goal_name.yaml"), "error: {err}");
        assert!(err.contains("name must not be empty"), "error: {err}");
    }

    #[test]
    fn non_positive_goal_target_fails_to_load() {
        let path = write_tmp(
            "ender_neg_target.yaml",
            "cash: 0
goals:
  - { name: college, target: 0, by_month: 0 }
events: []
",
        );
        let Err(err) = load(&path) else {
            panic!("expected load to fail")
        };
        assert!(err.contains("ender_neg_target.yaml"), "error: {err}");
        assert!(err.contains("target must be positive"), "error: {err}");
    }

    #[test]
    fn empty_goals_produces_no_extra_output() {
        // Run scenario.yaml-style content with and without an explicit empty
        // `goals:` field and confirm the text summary is byte-identical.
        let base = "cash: 20000
reserve_months: 3
events:
  - when: 0
    type: job
    salary: { monthly: 8000, annualized_rate: 0.03 }
  - when: 0
    type: expense
    rent: { monthly: 2000, annualized_rate: 0.02 }
";
        let mut a = serde_yaml_ng::from_str::<ScenarioInput>(base).unwrap().into_scenario().unwrap();
        let stats_a = a.run(12);
        let out_a = format_text_summary(&stats_a, a.saving_breach_month, &[], None);
        assert!(!out_a.contains("goal"), "no-goals output must not mention goals: {out_a}");

        let mut b = serde_yaml_ng::from_str::<ScenarioInput>(&format!("{base}goals: []\n"))
            .unwrap()
            .into_scenario().unwrap();
        let stats_b = b.run(12);
        let outcomes_b = evaluate_goals(&stats_b, &b.goals, None);
        let out_b = format_text_summary(&stats_b, b.saving_breach_month, &outcomes_b, None);
        assert_eq!(out_a, out_b, "empty goals list must not change the text summary");
    }

    #[test]
    fn saving_and_goals_coexist() {
        // Saving floor 500 (no breach: rents don't drop below), college goal
        // at month 2 met. Both lines appear in the summary.
        let mut s = scenario(
            "cash: 0
saving: 0
goals:
  - { name: college, target: 100, by_month: 2 }
events:
  - { when: 0, type: job, salary: { monthly: 1000, annualized_rate: 0.0 } }
  - { when: 0, type: expense, rent: { monthly: 200, annualized_rate: 0.0 } }",
        );
        let stats = s.run(2);
        // saving target 0 means breach only if cash < 0; cash stays positive.
        let outcomes = evaluate_goals(&stats, &s.goals, None);
        let out = format_text_summary(&stats, s.saving_breach_month, &outcomes, None);
        assert_eq!(s.saving_breach_month, None, "no saving breach expected");
        assert!(out.contains("goal college met"), "got: {out}");
        assert!(out.contains("at month 2"), "got: {out}");

        // Now exercise the actual coexistence path: saving breach + goal met.
        let mut s2 = scenario(
            "cash: 1000
saving: 500
goals:
  - { name: college, target: 100, by_month: 3 }
events:
  - { when: 0, type: expense, rent: { monthly: 1000, annualized_rate: 0.0 } }
  - { when: 1, type: one_off_income, one_off: { amount: 5000 } }",
        );
        let stats2 = s2.run(3);
        let outcomes2 = evaluate_goals(&stats2, &s2.goals, None);
        let out2 = format_text_summary(&stats2, s2.saving_breach_month, &outcomes2, None);
        assert_eq!(s2.saving_breach_month, Some(0), "saving should breach on month 0");
        assert!(out2.contains("saving target breached at month 0\n"), "got: {out2}");
        assert!(out2.contains("goal college met"), "got: {out2}");
    }

    #[test]
    fn json_goals_array_always_present() {
        // No goals: goals field is `[]`, wrapper shape preserved.
        let mut s = scenario("cash: 100\nevents: []");
        let stats = s.run(1);
        let outcomes = evaluate_goals(&stats, &s.goals, None);
        let parsed: serde_json::Value = serde_json::from_str(&stats_json(&stats, &outcomes, None)).unwrap();
        let goals = parsed["goals"].as_array().expect("goals field must be an array");
        assert_eq!(goals.len(), 0);

        // Two goals: populated array, ordered by by_month, correct types.
        let goals_in = vec![
            goal("retirement", 1_000_000.0, 3, GoalKind::NetWorth),
            goal("college", 200_000.0, 1, GoalKind::Cash),
        ];
        let stats2: Vec<Stats> = (0..=3)
            .map(|m| Stats {
                cash: if m == 1 { 250_000.0 } else { 500_000.0 },
                assets_value: if m == 3 { 700_000.0 } else { 0.0 },
                monthly_cashflow: 0.0,
            })
            .collect();
        let outcomes2 = evaluate_goals(&stats2, &goals_in, None);
        let parsed2: serde_json::Value = serde_json::from_str(&stats_json(&stats2, &outcomes2, None)).unwrap();
        let arr = parsed2["goals"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "college");
        assert_eq!(arr[0]["kind"], "cash");
        assert_eq!(arr[0]["target"].as_f64().unwrap(), 200_000.0);
        assert_eq!(arr[0]["by_month"].as_u64().unwrap(), 1);
        assert_eq!(arr[0]["met"].as_bool().unwrap(), true);
        assert_eq!(arr[0]["value"].as_f64().unwrap(), 250_000.0);
        assert_eq!(arr[1]["name"], "retirement");
        assert_eq!(arr[1]["kind"], "net_worth");
        assert_eq!(arr[1]["by_month"].as_u64().unwrap(), 3);
        // evaluation_month is not exposed in JSON
        assert!(arr[0].get("evaluation_month").is_none(), "evaluation_month must be skipped in JSON");
    }

    // ---- Event: downturn ----

    #[test]
    fn downturn_drops_fund_and_property_in_one_month() {
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: investment
    fund: { principal: 1000000, annualized_rate: 0.0 }
  - when: 0
    type: buy_home
    property: { sqft: 100, price_per_sqft: 30000, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 1 } } }
  - when: 0
    type: downturn
    equity_drop: 0.5
    property_drop: 0.5",
        );
        let stats = s.run(0);
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 500_000.0),
            _ => panic!("expected fund"),
        }
        match &s.assets[1] {
            Asset::Property { price_per_sqft, .. } => assert_eq!(*price_per_sqft, 15_000.0),
            _ => panic!("expected property"),
        }
        // monthly_cashflow unchanged: downturn is a balance-sheet move.
        assert_eq!(stats[0].monthly_cashflow, 0.0);
    }

    #[test]
    fn downturn_only_equity_leaves_property_untouched() {
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: investment
    fund: { principal: 1000, annualized_rate: 0.0 }
  - when: 0
    type: buy_home
    property: { sqft: 100, price_per_sqft: 30000, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 1 } } }
  - when: 0
    type: downturn
    equity_drop: 0.22
    property_drop: 0.0",
        );
        s.run(0);
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 780.0),
            _ => panic!("expected fund"),
        }
        match &s.assets[1] {
            Asset::Property { price_per_sqft, .. } => assert_eq!(*price_per_sqft, 30_000.0),
            _ => panic!("expected property"),
        }
    }

    #[test]
    fn downturn_does_not_refire() {
        // One downturn at month 0 with drop 0.5; running the loaded
        // scenario for 5 months must not re-apply the drop on later months.
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: investment
    fund: { principal: 1000, annualized_rate: 0.0 }
  - when: 0
    type: downturn
    equity_drop: 0.5",
        );
        let stats = s.run(5);
        match &s.assets[0] {
            Asset::Fund { principal, .. } => assert_eq!(*principal, 500.0),
            _ => panic!("expected fund"),
        }
        // After the drop, monthly_cashflow stays at zero (no flow).
        assert!(stats[1].monthly_cashflow.abs() < 0.01);
        assert!(stats[5].monthly_cashflow.abs() < 0.01);
    }

    // ---- Event: refinance ----

    #[test]
    fn refinance_updates_mortgage_monthly() {
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: buy_home
    property: { sqft: 100, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { id: home-loan, mortgage: { monthly: 1000, period: 240 } } }
  - when: 0
    type: refinance
    id: home-loan
    monthly: 2000",
        );
        s.run(1);
        match &s.assets[0] {
            Asset::Property { mortgage, .. } => match &mortgage.item {
                CashflowItem::Mortgage { monthly, period, .. } => {
                    assert_eq!(*monthly, 2000.0);
                    assert_eq!(*period, 240);
                }
                _ => panic!("expected mortgage"),
            },
            _ => panic!("expected property"),
        }
    }

    #[test]
    fn refinance_unknown_id_is_noop() {
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: buy_home
    property: { sqft: 100, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { id: home-loan, mortgage: { monthly: 1000, period: 240 } } }
  - when: 0
    type: refinance
    id: ghost
    monthly: 9999",
        );
        s.run(0);
        match &s.assets[0] {
            Asset::Property { mortgage, .. } => match &mortgage.item {
                CashflowItem::Mortgage { monthly, .. } => assert_eq!(*monthly, 1000.0),
                _ => panic!("expected mortgage"),
            },
            _ => panic!("expected property"),
        }
    }

    #[test]
    fn refinance_without_id_fails_to_load() {
        let yaml = "cash: 0\nevents:\n  - { when: 0, type: refinance, monthly: 1000 }\n";
        let err = serde_yaml_ng::from_str::<ScenarioInput>(yaml).unwrap_err().to_string();
        assert!(err.contains("id"), "error should mention id: {err}");
    }

    // ---- Event: downturn rent_drop ----

    #[test]
    fn downturn_rent_drop_lowers_rental_income() {
        let mut s = scenario(
            "cash: 0
events:
  - when: 0
    type: buy_to_let
    property: { sqft: 100, price_per_sqft: 100, capex_per_sqft: 0, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 0, period: 1 } }, rental: { rental_income: { occupancy: 1.0, monthly: 30000, annualized_rate: 0.0 } } }
  - when: 0
    type: downturn
    equity_drop: 0.0
    property_drop: 0.0
    rent_drop: 0.10",
        );
        s.run(0);
        match &s.assets[0] {
            Asset::Property { rental: Some(r), .. } => match &r.item {
                CashflowItem::RentalIncome { monthly, .. } => assert_eq!(*monthly, 27_000.0),
                _ => panic!("expected rental income"),
            },
            _ => panic!("expected property with rental"),
        }
    }

    // ---- Event: pay_change ----

    #[test]
    fn pay_change_updates_first_salary_when_no_id() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, salary: { monthly: 100000, annualized_rate: 0.03 } }
  - { when: 0, type: job, salary: { monthly: 50000, annualized_rate: 0.03 } }
  - { when: 6, type: pay_change, monthly: 200000, annualized_rate: 0.0 }",
        );
        s.run(7);
        match &s.cashflow[0].item {
            CashflowItem::Salary { monthly, annualized_rate } => {
                assert!(*monthly > 100_000.0 && *monthly <= 200_000.0,
                    "monthly {monthly} should be at or below the new 200k after some compounding");
                assert_eq!(*annualized_rate, 0.0);
            }
            _ => panic!("expected salary"),
        }
    }

    #[test]
    fn pay_change_with_id_targets_labeled_salary() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 0, type: job, id: me, salary: { monthly: 100000, annualized_rate: 0.03 } }
  - { when: 0, type: job, id: partner, salary: { monthly: 50000, annualized_rate: 0.03 } }
  - { when: 6, type: pay_change, id: partner, monthly: 80000, annualized_rate: 0.0 }",
        );
        s.run(7);
        for c in &s.cashflow {
            if c.id.as_deref() == Some("partner") {
                if let CashflowItem::Salary { monthly, annualized_rate } = &c.item {
                    assert!(*monthly > 50_000.0 && *monthly <= 80_000.0);
                    assert_eq!(*annualized_rate, 0.0);
                }
            }
        }
    }

    // ---- Event: death (terminal scenario) ----

    #[test]
    fn death_event_terminates_run() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 60, type: death }",
        );
        let stats = s.run(360);
        assert_eq!(stats.len(), 61); // months 0..=60
        assert!(s.terminated);
    }

    #[test]
    fn death_with_no_other_events_produces_one_month() {
        let mut s = scenario("cash: 0\nevents:\n  - { when: 0, type: death }");
        let stats = s.run(360);
        assert_eq!(stats.len(), 1);
        assert!(s.terminated);
    }

    #[test]
    fn scheduled_payout_fires_at_death_month() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 24, type: death }
  - { when: 24, type: one_off_income, one_off: { amount: 1000 } }",
        );
        let stats = s.run(360);
        assert_eq!(stats[24].monthly_cashflow, 1000.0);
        assert!(s.terminated);
        // the death month is terminal — no stats past 24
        assert_eq!(stats.len(), 25);
    }

    #[test]
    fn no_death_event_runs_to_horizon_unchanged() {
        let mut s = scenario(
            "cash: 100
events:
  - { when: 0, type: expense, rent: { monthly: 0, annualized_rate: 0.0 } }",
        );
        let stats = s.run(5);
        assert_eq!(stats.len(), 6);
        assert!(!s.terminated);
    }

    #[test]
    fn text_summary_reports_termination() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 12, type: death }",
        );
        let stats = s.run(360);
        let out = format_text_summary(&stats, None, &[], Some(12));
        assert!(
            out.ends_with("terminal at month 12 (death event)\n"),
            "got: {out}"
        );
    }

    #[test]
    fn text_summary_omits_termination_when_no_death_event() {
        let stats = vec![s(100.0, 50.0, 25.0), s(75.0, 60.0, -25.0)];
        let out = format_text_summary(&stats, None, &[], None);
        assert!(!out.contains("terminal"), "got: {out}");
        assert_eq!(
            out,
            "after 1 months: cash 75.00, assets 60.00, monthly cashflow -25.00\n"
        );
    }

    #[test]
    fn json_output_wraps_array() {
        let yaml = "cash: 0\nevents:\n  - { when: 24, type: death }\n  - { when: 24, type: one_off_income, one_off: { amount: 1000 } }\n";
        let mut s = scenario(yaml);
        let stats = s.run(360);
        let parsed: serde_json::Value =
            serde_json::from_str(&stats_json(&stats, &[], Some(24))).unwrap();
        assert_eq!(parsed["terminal"], true);
        assert_eq!(parsed["terminal_month"], 24);
        let months = parsed["months"].as_array().unwrap();
        assert_eq!(months.len(), 25);

        // non-terminated run
        let mut s2 = scenario("cash: 100\nevents: []");
        let stats2 = s2.run(3);
        let parsed2: serde_json::Value =
            serde_json::from_str(&stats_json(&stats2, &[], None)).unwrap();
        assert_eq!(parsed2["terminal"], false);
        assert!(parsed2.get("terminal_month").is_none());
        assert_eq!(parsed2["months"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn json_per_month_shape_unchanged() {
        let mut s = scenario(
            "cash: 0
events:
  - { when: 60, type: death }",
        );
        let stats = s.run(360);
        let parsed: serde_json::Value =
            serde_json::from_str(&stats_json(&stats, &[], Some(60))).unwrap();
        for obj in parsed["months"].as_array().unwrap() {
            assert!(obj.get("month").is_some());
            assert!(obj.get("cash").is_some());
            assert!(obj.get("assets_value").is_some());
            assert!(obj.get("monthly_cashflow").is_some());
        }
    }
}
