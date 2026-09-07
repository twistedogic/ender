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
    fn monthly(&mut self) -> f64 {
        match self {
            Self::Empty => 0.0,
            Self::Investment { monthly } => -*monthly,
            Self::Rent {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                -m
            }
            Self::Salary {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                m
            }
            Self::RentalIncome {
                occupancy,
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                m * *occupancy
            }
            Self::Mortgage {
                period,
                paid,
                monthly,
            } => {
                if paid >= period {
                    return 0.0;
                }
                *paid += 1;
                -*monthly
            }
            Self::Tuition {
                monthly,
                annualized_rate,
            } => {
                let m = monthly.clone();
                *monthly *= (1.0 + *annualized_rate).powf(1.0 / 12.0);
                -m
            }
        }
    }
}

enum Asset {
    Fund {
        principal: f64,
        annualized_rate: f64,
        investment: Option<CashflowItem>,
    },
    Property {
        sqrt_ft: f64,
        price_per_sqrt_ft: f64,
        capex_per_sqrt_ft: f64,
        annualized_rate: f64,
        mortgage: CashflowItem,
        rental: Option<CashflowItem>,
    },
}

impl Asset {
    fn value(self) -> f64 {
        match self {
            Self::Fund { principal, .. } => principal,
            Self::Property {
                sqrt_ft,
                price_per_sqrt_ft,
                ..
            } => sqrt_ft * price_per_sqrt_ft,
        }
    }

    fn monthly(&mut self) -> f64 {
        match self {
            Self::Property {
                price_per_sqrt_ft,
                capex_per_sqrt_ft,
                annualized_rate,
                mortgage,
                rental,
                ..
            } => {
                let rent_income = match rental {
                    Some(r) => r.monthly(),
                    None => 0.0,
                };
                let rate = (1.0 + *annualized_rate).powf(1.0 / 12.0);
                *price_per_sqrt_ft *= rate;
                *capex_per_sqrt_ft *= rate;
                rent_income + mortgage.monthly()
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
                    None => 0.0,
                };
                *principal += m;
                m
            }
        }
    }
}

enum EventType {
    Job(CashflowItem),
    Expense(CashflowItem),
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
        }
    }
}

struct Event {
    when: u16,
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
        for e in self.events {
            if e.when == self.at {
                e.kind.apply(self);
            }
        }
        let cashflow = self.cashflow.iter_mut().map(|flow| flow.monthly()).sum()
            + self.assets.iter_mut().map(|flow| flow.monthly()).sum();
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

fn main() {
    println!("Hello, world!");
}
