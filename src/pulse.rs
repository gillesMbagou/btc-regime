use anyhow::{Context, Result};
use colored::{ColoredString, Colorize};
use serde::{Deserialize, Deserializer};

const USER_AGENT: &str = "Mozilla/5.0 (compatible; btc-regime/0.1)";
const BASE: &str = "https://fapi.binance.com";

fn de_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    String::deserialize(deserializer)?
        .parse()
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
struct PremiumIndexResp {
    #[serde(rename = "markPrice", deserialize_with = "de_f64")]
    mark_price: f64,
    #[serde(rename = "lastFundingRate", deserialize_with = "de_f64")]
    last_funding_rate: f64,
}

pub struct Snapshot {
    pub mark_price: f64,
    /// Fraction per 8h funding interval (e.g. 0.0001 = 0.01%/8h), not annualized.
    pub funding_rate: f64,
}

/// Current mark price and funding rate for a USDT-margined perpetual (e.g. "BTCUSDT").
pub fn fetch_premium_index(symbol: &str) -> Result<Snapshot> {
    let url = format!("{BASE}/fapi/v1/premiumIndex?symbol={symbol}");
    let resp: PremiumIndexResp = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .with_context(|| format!("échec de la requête Binance premiumIndex pour {symbol}"))?
        .error_for_status()
        .with_context(|| format!("Binance a répondu avec une erreur pour {symbol}"))?
        .json()
        .with_context(|| format!("réponse Binance premiumIndex illisible pour {symbol}"))?;
    Ok(Snapshot {
        mark_price: resp.mark_price,
        funding_rate: resp.last_funding_rate,
    })
}

#[derive(Deserialize)]
struct OiHistPoint {
    #[serde(rename = "sumOpenInterestValue", deserialize_with = "de_f64")]
    sum_open_interest_value: f64,
}

/// Daily USD open interest snapshots for the last `days` days, oldest first.
/// Binance only retains ~30 days of history for this endpoint.
pub fn fetch_oi_history(symbol: &str, days: u32) -> Result<Vec<f64>> {
    let url =
        format!("{BASE}/futures/data/openInterestHist?symbol={symbol}&period=1d&limit={days}");
    let resp: Vec<OiHistPoint> = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .with_context(|| format!("échec de la requête Binance openInterestHist pour {symbol}"))?
        .error_for_status()
        .with_context(|| format!("Binance a répondu avec une erreur pour {symbol}"))?
        .json()
        .with_context(|| format!("réponse Binance openInterestHist illisible pour {symbol}"))?;
    Ok(resp
        .into_iter()
        .map(|p| p.sum_open_interest_value)
        .collect())
}

/// Annualizes a per-8h funding rate (3 payments/day, 365 days).
pub fn annualized_funding(rate_per_8h: f64) -> f64 {
    rate_per_8h * 3.0 * 365.0
}

pub fn pct_change(from: f64, to: f64) -> f64 {
    (to - from) / from
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Froth {
    /// Positive, elevated funding while open interest is rising: leverage building up.
    Building,
    /// Negative funding, or open interest falling meaningfully: leverage flushing out.
    Resetting,
    Neutral,
}

/// Reads leverage conditions the way the newsletter's "So What?" sections do:
/// hot positive funding alongside rising OI is a crowded long book building;
/// negative funding or a real OI drawdown is a flush/reset rather than a trend break.
pub fn classify(funding_rate_per_8h: f64, oi_change_pct: f64) -> Froth {
    const HOT_FUNDING: f64 = 0.0001; // 0.01%/8h, ~11% annualized — a common "elevated" bar
    const OI_MOVE: f64 = 0.03; // 3% swing over the lookback window

    if funding_rate_per_8h > HOT_FUNDING && oi_change_pct > OI_MOVE {
        Froth::Building
    } else if funding_rate_per_8h < -HOT_FUNDING || oi_change_pct < -OI_MOVE {
        Froth::Resetting
    } else {
        Froth::Neutral
    }
}

fn fmt_froth(f: Froth) -> ColoredString {
    match f {
        Froth::Building => "levier en construction".yellow(),
        Froth::Resetting => "purge / reset de levier".cyan(),
        Froth::Neutral => "neutre".normal(),
    }
}

pub struct AssetPulse {
    pub symbol: &'static str,
    pub mark_price: f64,
    pub funding_rate: f64,
    pub oi_now: f64,
    pub oi_change_pct: f64,
    pub froth: Froth,
}

impl AssetPulse {
    fn print(&self) {
        println!("\n  {}", self.symbol.bold());
        println!("    {:<24} ${:>14.2}", "Prix (mark):", self.mark_price);
        println!(
            "    {:<24} {:>+.4}%/8h  ({:>+.1}% annualisé)",
            "Funding rate:",
            self.funding_rate * 100.0,
            annualized_funding(self.funding_rate) * 100.0
        );
        println!(
            "    {:<24} ${:>13.2}B  ({:>+.1}% sur la période)",
            "Open interest:",
            self.oi_now / 1e9,
            self.oi_change_pct * 100.0
        );
        println!("    {:<24} {}", "Lecture:", fmt_froth(self.froth));
    }
}

pub struct PulseReport {
    pub assets: Vec<AssetPulse>,
}

impl PulseReport {
    pub fn print(&self) {
        println!(
            "\n{}",
            "Market Pulse — funding rate & open interest (Binance Futures)".bold()
        );
        println!("{}", "─".repeat(70).dimmed());
        for asset in &self.assets {
            asset.print();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_change_up_and_down() {
        assert!((pct_change(100.0, 110.0) - 0.10).abs() < 1e-9);
        assert!((pct_change(100.0, 90.0) + 0.10).abs() < 1e-9);
    }

    #[test]
    fn annualized_funding_matches_convention() {
        // 0.01%/8h * 3/day * 365 = 10.95%/year
        assert!((annualized_funding(0.0001) - 0.1095).abs() < 1e-6);
    }

    #[test]
    fn classify_hot_funding_and_rising_oi_is_building() {
        assert_eq!(classify(0.0005, 0.10), Froth::Building);
    }

    #[test]
    fn classify_negative_funding_is_resetting() {
        assert_eq!(classify(-0.0002, 0.01), Froth::Resetting);
    }

    #[test]
    fn classify_oi_drawdown_is_resetting_even_with_positive_funding() {
        assert_eq!(classify(0.00005, -0.08), Froth::Resetting);
    }

    #[test]
    fn classify_calm_conditions_are_neutral() {
        assert_eq!(classify(0.00002, 0.005), Froth::Neutral);
    }
}
