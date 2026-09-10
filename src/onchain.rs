use anyhow::{Context, Result};
use colored::{ColoredString, Colorize};
use serde::Deserialize;

const USER_AGENT: &str = "Mozilla/5.0 (compatible; btc-regime/0.1)";
const POOLS_URL: &str = "https://yields.llama.fi/pools";

#[derive(Deserialize)]
struct PoolsResponse {
    data: Vec<Pool>,
}

#[derive(Deserialize)]
struct Pool {
    project: String,
    symbol: String,
    chain: String,
    apy: Option<f64>,
    #[serde(rename = "tvlUsd")]
    tvl_usd: f64,
}

pub struct PoolQuery<'a> {
    pub project: &'a str,
    pub chain: &'a str,
    pub symbol: &'a str,
}

pub struct PoolRate {
    /// Annual percentage yield, in percentage points (e.g. 3.71 = 3.71%/year).
    pub apy_pct: f64,
    pub tvl_usd: f64,
}

/// Fetches DeFiLlama's full pool list once and resolves each query to its
/// highest-TVL match — the main lending market rather than a thin isolated one.
pub fn fetch_stablecoin_apys(queries: &[PoolQuery]) -> Result<Vec<PoolRate>> {
    let resp: PoolsResponse = reqwest::blocking::Client::new()
        .get(POOLS_URL)
        .header("User-Agent", USER_AGENT)
        .send()
        .context("échec de la requête DeFiLlama pools")?
        .error_for_status()
        .context("DeFiLlama a répondu avec une erreur")?
        .json()
        .context("réponse DeFiLlama illisible")?;

    queries
        .iter()
        .map(|q| {
            resp.data
                .iter()
                .filter(|p| p.project == q.project && p.chain == q.chain && p.symbol == q.symbol)
                .filter_map(|p| {
                    p.apy.map(|apy_pct| PoolRate {
                        apy_pct,
                        tvl_usd: p.tvl_usd,
                    })
                })
                .max_by(|a, b| a.tvl_usd.total_cmp(&b.tvl_usd))
                .with_context(|| {
                    format!(
                        "aucun pool {}/{}/{} trouvé sur DeFiLlama",
                        q.project, q.chain, q.symbol
                    )
                })
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SpreadRead {
    /// Onchain APY beats the risk-free rate: a positive premium for smart
    /// contract, peg and counterparty risk.
    Rational,
    /// Onchain APY is below the risk-free rate: lending pays less than doing
    /// nothing in a T-bill, before even weighing the extra risk.
    Irrational,
}

pub fn classify_spread(apy_pct: f64, risk_free_pct: f64) -> SpreadRead {
    if apy_pct > risk_free_pct {
        SpreadRead::Rational
    } else {
        SpreadRead::Irrational
    }
}

fn fmt_read(r: SpreadRead) -> ColoredString {
    match r {
        SpreadRead::Rational => "prime positive — prêter est rationnel".green(),
        SpreadRead::Irrational => "prime négative — le rendement ne compense pas le risque".red(),
    }
}

pub struct StablecoinLine {
    pub label: &'static str,
    pub apy_pct: f64,
    pub tvl_usd: f64,
    pub spread_pct: f64,
    pub read: SpreadRead,
}

impl StablecoinLine {
    fn print(&self) {
        println!("\n  {}", self.label.bold());
        println!("    {:<24} {:>6.2}%", "APY offert:", self.apy_pct);
        println!("    {:<24} ${:>9.1}M", "TVL du pool:", self.tvl_usd / 1e6);
        println!(
            "    {:<24} {:>+6.2} pts",
            "Écart vs sans risque:", self.spread_pct
        );
        println!("    {:<24} {}", "Lecture:", fmt_read(self.read));
    }
}

pub struct OnchainReport {
    pub risk_free_pct: f64,
    pub risk_free_label: &'static str,
    pub lines: Vec<StablecoinLine>,
}

impl OnchainReport {
    pub fn print(&self) {
        println!(
            "\n{}",
            "On-Chain Yield — prêt stablecoin vs taux sans risque".bold()
        );
        println!("{}", "─".repeat(70).dimmed());
        println!(
            "  {:<24} {:>6.2}%",
            format!("Taux sans risque ({}):", self.risk_free_label),
            self.risk_free_pct
        );
        for line in &self.lines {
            line.print();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apy_above_risk_free_is_rational() {
        assert_eq!(classify_spread(4.2, 3.85), SpreadRead::Rational);
    }

    #[test]
    fn apy_below_risk_free_is_irrational() {
        assert_eq!(classify_spread(3.5, 3.85), SpreadRead::Irrational);
    }

    #[test]
    fn apy_equal_to_risk_free_is_irrational() {
        // No premium for the extra risk still doesn't compensate for it.
        assert_eq!(classify_spread(3.85, 3.85), SpreadRead::Irrational);
    }
}
