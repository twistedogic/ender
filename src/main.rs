use serde::Deserialize;

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
    Empty,
}

impl Default for CashflowItem {
    fn default() -> Self {
        Self::Empty
    }
}

impl CashflowItem {
    fn monthly(&mut self) -> Cashflow {
        match self {
            Self::Empty => Cashflow::Income(0.0),
            Self::Investment { monthly } => Cashflow::Expense(*monthly),
            Self::Rent {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                Cashflow::Expense(m)
            }
            Self::Salary {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                Cashflow::Income(m)
            }
            Self::RentalIncome {
                occupancy,
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                Cashflow::Income(m * *occupancy)
            }
            Self::Mortgage {
                period,
                paid,
                monthly,
            } => {
                if paid >= period {
                    return Cashflow::Expense(0.0);
                }
                *paid += 1;
                Cashflow::Expense(*monthly)
            }
            Self::Tuition {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                Cashflow::Expense(m)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Asset {
    Fund {
        principal: f64,
        annualized_rate: f64,
        investment: Option<CashflowItem>,
    },
    Property {
        sqft: f64,
        price_per_sqft: f64,
        capex_per_sqft: f64,
        annualized_rate: f64,
        mortgage: CashflowItem,
        rental: Option<CashflowItem>,
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

    fn monthly(&mut self) -> Cashflow {
        match self {
            Self::Property {
                price_per_sqft,
                capex_per_sqft,
                annualized_rate,
                mortgage,
                rental,
                ..
            } => {
                let rent_income = match rental {
                    Some(r) => r.monthly(),
                    None => Cashflow::Income(0.0),
                };
                let rate = (1.0 + *annualized_rate).powf(1.0 / 12.0);
                *price_per_sqft *= rate;
                *capex_per_sqft *= rate;
                rent_income.add(mortgage.monthly())
            }
            Self::Fund {
                principal,
                annualized_rate,
                investment,
            } => {
                let rate = (1.0 + *annualized_rate).powf(1.0 / 12.0);
                *principal *= rate;
                let m = match investment {
                    Some(i) => i.monthly(),
                    None => Cashflow::Expense(0.0),
                };
                if let Cashflow::Expense(v) = m {
                    *principal += v;
                }
                m
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EventType {
    Job(CashflowItem),
    Expense(CashflowItem),
    Inition(CashflowItem),
    Graduate,
    Layoff,
    BuyHome(Asset),
    BuyToLet(Asset),
    Investment(Asset),
}

impl EventType {
    fn apply(self, s: &mut Scenario) {
        match self {
            Self::Job(i) => s.cashflow.push(i),
            Self::Layoff => {
                if let Some(idx) = s.cashflow.iter().position(|x| match x {
                    CashflowItem::Salary { .. } => true,
                    _ => false,
                }) {
                    s.cashflow.remove(idx);
                }
            }
            Self::Expense(e) => s.cashflow.push(e),
            Self::BuyHome(a) => s.assets.push(a),
            Self::BuyToLet(a) => s.assets.push(a),
            Self::Investment(a) => s.assets.push(a),
            Self::Inition(i) => s.cashflow.push(i),
            Self::Graduate => {
                if let Some(idx) = s.cashflow.iter().position(|x| match x {
                    CashflowItem::Tuition { .. } => true,
                    _ => false,
                }) {
                    s.cashflow.remove(idx);
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

struct Stats {
    cash: f64,
    assets_value: f64,
    monthly_cashflow: f64,
}

struct Scenario {
    at: u16,
    cash: f64,
    cashflow: Vec<CashflowItem>,
    assets: Vec<Asset>,
    events: Vec<Event>,
}

impl Scenario {
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
        let cashflow = self
            .cashflow
            .iter_mut()
            .map(|flow| flow.monthly().value())
            .sum::<f64>()
            + self
                .assets
                .iter_mut()
                .map(|flow| flow.monthly().value())
                .sum::<f64>();
        let assets_value = self.assets.iter_mut().map(|a| a.value()).sum();
        self.cash += cashflow;
        self.at += 1;
        Stats {
            cash: self.cash,
            assets_value,
            monthly_cashflow: cashflow,
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
        }
    }
}

fn load(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let input: ScenarioInput =
        serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(input.into_scenario())
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "scenario.yaml".to_string());
    let mut scenario = match load(std::path::Path::new(&path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let stats = scenario.run(360);
    let last = stats.last().unwrap();
    println!(
        "after 360 months: cash {:.2}, assets {:.2}, monthly cashflow {:.2}",
        last.cash, last.assets_value, last.monthly_cashflow
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expense_event_fires_exactly_once() {
        let mut s = Scenario {
            at: 0,
            cash: 0.0,
            cashflow: Vec::new(),
            assets: Vec::new(),
            events: vec![Event {
                when: 3,
                kind: EventType::Expense(CashflowItem::Rent {
                    monthly: 2000.0,
                    annualized_rate: 0.0,
                }),
            }],
        };
        let stats = s.run(12);
        assert_eq!(stats.len(), 13);
        assert_eq!(stats[2].monthly_cashflow, 0.0);
        assert_eq!(stats[3].monthly_cashflow, -2000.0);
        assert_eq!(stats[11].monthly_cashflow, -2000.0);
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
    salary: { monthly: 8000, annualized_rate: 0.03 }
  - when: 0
    type: expense
    rent: { monthly: 2000, annualized_rate: 0.02 }
  - when: 6
    type: layoff
  - when: 9
    type: investment
    fund: { principal: 1000, annualized_rate: 0.05, investment: { investment: { monthly: 500 } } }
  - when: 12
    type: buy_home
    property: { sqft: 1200, price_per_sqft: 300, capex_per_sqft: 20, annualized_rate: 0.03, mortgage: { mortgage: { monthly: 1800, period: 240 } } }
  - when: 24
    type: buy_to_let
    property: { sqft: 900, price_per_sqft: 200, capex_per_sqft: 15, annualized_rate: 0.02, mortgage: { mortgage: { monthly: 1200, period: 120 } }, rental: { rental_income: { occupancy: 0.85, monthly: 2200, annualized_rate: 0.02 } } }
"#;
        let mut s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario();
        let stats = s.run(24);
        assert_eq!(stats.len(), 25);
        // salary fired then was removed by layoff; rent remains
        assert_eq!(s.cashflow.len(), 1);
        assert!(matches!(&s.cashflow[0], CashflowItem::Rent { monthly, .. } if *monthly > 2000.0));
        // fund + home + rental property
        assert_eq!(s.assets.len(), 3);
    }

    #[test]
    fn mortgage_paid_defaults_to_zero() {
        let yaml = "cash: 0\nevents:\n  - when: 0\n    type: buy_home\n    property: { sqft: 1000, price_per_sqft: 100, capex_per_sqft: 10, annualized_rate: 0.0, mortgage: { mortgage: { monthly: 1000, period: 12 } } }";
        let s = serde_yaml_ng::from_str::<ScenarioInput>(yaml)
            .unwrap()
            .into_scenario();
        match &s.events[0].kind {
            EventType::BuyHome(Asset::Property {
                mortgage: CashflowItem::Mortgage { paid, period, .. },
                ..
            }) => {
                assert_eq!(*paid, 0);
                assert_eq!(*period, 12);
            }
            _ => panic!("expected mortgaged property"),
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
}
