use serde::{Deserialize, Serialize};

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

#[derive(Deserialize)]
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
        period: u8,
        #[serde(default)]
        paid: u8,
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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
    },
}

impl Asset {
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

#[derive(Deserialize)]
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
                s.cash -= a.value();
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
        }
    }
}

#[derive(Deserialize)]
struct Event {
    when: u16,
    #[serde(flatten)]
    kind: EventType,
}

#[derive(Serialize)]
struct Stats {
    cash: f64,
    assets_value: f64,
    monthly_cashflow: f64,
}

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
            self.cash -= salaries_tax(self.salary_accrued, self.mpf_accrued, BASIC_ALLOWANCE)
                + property_tax(self.rent_accrued);
            self.salary_accrued = 0.0;
            self.mpf_accrued = 0.0;
            self.rent_accrued = 0.0;
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
            stats.push(self.once())
        }
        stats
    }
}

#[derive(Deserialize)]
struct ScenarioInput {
    cash: f64,
    #[serde(default)]
    reserve_months: u32,
    events: Vec<Event>,
}

impl ScenarioInput {
    fn into_scenario(self) -> Scenario {
        Scenario {
            at: 0,
            cash: self.cash,
            cashflow: Vec::new(),
            assets: Vec::new(),
            events: self.events,
            reserve_months: self.reserve_months,
            salary_accrued: 0.0,
            mpf_accrued: 0.0,
            rent_accrued: 0.0,
        }
    }
}

fn load(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let input: ScenarioInput =
        serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(input.into_scenario())
}

/// Parse CLI args: first non-flag is the path (default `scenario.yaml`),
/// `--json` may appear anywhere, any other `-`-prefixed arg is an error.
fn parse_args(args: &[String]) -> Result<(String, bool), String> {
    let mut path: Option<String> = None;
    let mut json = false;
    for a in args {
        if a == "--json" {
            json = true;
        } else if a.starts_with('-') {
            return Err(format!("unknown flag: {a}"));
        } else if path.is_none() {
            path = Some(a.clone());
        }
    }
    Ok((path.unwrap_or_else(|| "scenario.yaml".to_string()), json))
}

/// Serialize the stats series as a compact JSON array: one object per
/// simulated month with `month`, `cash`, `assets_value`, `monthly_cashflow`.
fn stats_json(stats: &[Stats]) -> String {
    #[derive(Serialize)]
    struct Row<'a> {
        month: usize,
        #[serde(flatten)]
        stats: &'a Stats,
    }
    let rows: Vec<Row> = stats.iter().enumerate().map(|(month, s)| Row { month, stats: s }).collect();
    serde_json::to_string(&rows).expect("stats JSON never fails")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, json) = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let mut scenario = match load(std::path::Path::new(&path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let stats = scenario.run(360);
    if json {
        println!("{}", stats_json(&stats));
        return;
    }
    let last = stats.last().unwrap();
    println!(
        "after 360 months: cash {:.2}, assets {:.2}, monthly cashflow {:.2}",
        last.cash, last.assets_value, last.monthly_cashflow
    );
    if let Some((month, _)) = stats.iter().enumerate().find(|(_, s)| s.cash < 0.0) {
        println!("insolvent from month {month}");
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
        let stats = s.run(2);
        assert_eq!(stats[0].cash, 19900.0); // 50000 - 30000 price - 100 mortgage
        assert_eq!(stats[1].cash - stats[0].cash, -100.0);
        assert_eq!(stats[2].cash - stats[1].cash, -100.0);
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
        let stats = s.run(3);
        assert_eq!(stats[3].cash, 100.0);
        assert_eq!(stats[3].monthly_cashflow, 0.0);
    }

    #[test]
    fn one_off_expense_lands_once() {
        let yaml = "cash: 20000\nevents:\n  - { when: 3, type: one_off_expense, one_off: { amount: 5000 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario();
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
            .into_scenario();
        let stats = s.run(1);
        assert_eq!(stats[0].monthly_cashflow, -100.0);
        assert_eq!(stats[1].monthly_cashflow, 0.0);
    }

    #[test]
    fn one_off_income_lands_once() {
        let yaml = "cash: 0\nevents:\n  - { when: 6, type: one_off_income, one_off: { amount: 10000 } }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario();
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
            .into_scenario();
        let stats = s.run(0);
        assert_eq!(stats[0].monthly_cashflow, -4000.0);
    }

    #[test]
    fn end_removes_a_one_off_before_it_fires() {
        let yaml = "cash: 20000\nevents:\n  - { when: 0, type: one_off_expense, id: tax, one_off: { amount: 5000 } }\n  - { when: 0, type: end, id: tax }\n";
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario();
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
        let s = input.into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario();
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
            .into_scenario()
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
        let (path, json) = parse_args(&[]).unwrap();
        assert_eq!(path, "scenario.yaml");
        assert!(!json);
    }

    #[test]
    fn parse_args_takes_explicit_path() {
        let (path, json) = parse_args(&["s.yaml".to_string()]).unwrap();
        assert_eq!(path, "s.yaml");
        assert!(!json);
    }

    #[test]
    fn parse_args_recognizes_json_alone() {
        let (path, json) = parse_args(&["--json".to_string()]).unwrap();
        assert_eq!(path, "scenario.yaml");
        assert!(json);
    }

    #[test]
    fn parse_args_recognizes_json_before_path() {
        let (path, json) = parse_args(&["--json".to_string(), "s.yaml".to_string()]).unwrap();
        assert_eq!(path, "s.yaml");
        assert!(json);
    }

    #[test]
    fn parse_args_recognizes_json_after_path() {
        let (path, json) = parse_args(&["s.yaml".to_string(), "--json".to_string()]).unwrap();
        assert_eq!(path, "s.yaml");
        assert!(json);
    }

    #[test]
    fn parse_args_rejects_unknown_flag_naming_it() {
        let err = parse_args(&["--yaml".to_string()]).unwrap_err();
        assert!(err.contains("--yaml"), "error should name the flag: {err}");
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
            .into_scenario();
        let stats = s.run(12);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&stats_json(&stats)).unwrap();
        assert_eq!(parsed.len(), 13);
        for (i, obj) in parsed.iter().enumerate() {
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
            .into_scenario();
        let stats = s.run(12);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&stats_json(&stats)).unwrap();
        let last = parsed.last().unwrap();
        let s_last = stats.last().unwrap();
        assert_eq!(last["cash"].as_f64().unwrap(), s_last.cash);
        assert_eq!(last["assets_value"].as_f64().unwrap(), s_last.assets_value);
        assert_eq!(
            last["monthly_cashflow"].as_f64().unwrap(),
            s_last.monthly_cashflow
        );
    }

}
