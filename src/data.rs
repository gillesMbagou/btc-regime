use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;

use crate::series::Series;

const USER_AGENT: &str = "Mozilla/5.0 (compatible; btc-regime/0.1)";

#[derive(Deserialize)]
struct CoinGeckoResponse {
    prices: Vec<[f64; 2]>,
}

/// Daily BTC/USD closes from CoinGecko's free public API (max 365 days of history).
pub fn fetch_btc(days: u32) -> Result<Series> {
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days={days}"
    );
    let resp: CoinGeckoResponse = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .context("échec de la requête CoinGecko")?
        .error_for_status()
        .context("CoinGecko a répondu avec une erreur")?
        .json()
        .context("réponse CoinGecko illisible")?;

    let series = resp
        .prices
        .into_iter()
        .filter_map(|[ts_ms, price]| {
            let date = Utc
                .timestamp_millis_opt(ts_ms as i64)
                .single()?
                .date_naive();
            Some((date, price))
        })
        .collect();

    Ok(dedup_by_date(series))
}

#[derive(Deserialize)]
struct YahooResponse {
    chart: YahooChart,
}
#[derive(Deserialize)]
struct YahooChart {
    result: Vec<YahooResult>,
}
#[derive(Deserialize)]
struct YahooResult {
    timestamp: Vec<i64>,
    indicators: YahooIndicators,
}
#[derive(Deserialize)]
struct YahooIndicators {
    quote: Vec<YahooQuote>,
}
#[derive(Deserialize)]
struct YahooQuote {
    close: Vec<Option<f64>>,
}

/// Daily closes for a Yahoo Finance symbol (e.g. "%5EIXIC" for the Nasdaq Composite,
/// "GC=F" for the Comex gold front future). No API key required.
pub fn fetch_yahoo(symbol: &str, range: &str) -> Result<Series> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range={range}&interval=1d"
    );
    let resp: YahooResponse = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .with_context(|| format!("échec de la requête Yahoo Finance pour {symbol}"))?
        .error_for_status()
        .with_context(|| format!("Yahoo Finance a répondu avec une erreur pour {symbol}"))?
        .json()
        .with_context(|| format!("réponse Yahoo Finance illisible pour {symbol}"))?;

    let result = resp
        .chart
        .result
        .into_iter()
        .next()
        .with_context(|| format!("aucune donnée Yahoo Finance pour {symbol}"))?;

    let closes = result
        .indicators
        .quote
        .into_iter()
        .next()
        .map(|q| q.close)
        .unwrap_or_default();

    let series = result
        .timestamp
        .into_iter()
        .zip(closes)
        .filter_map(|(ts, close)| {
            let close = close?;
            let date = Utc.timestamp_opt(ts, 0).single()?.date_naive();
            Some((date, close))
        })
        .collect();

    Ok(dedup_by_date(series))
}

fn dedup_by_date(mut series: Series) -> Series {
    series.sort_by_key(|(d, _)| *d);
    series.dedup_by_key(|(d, _)| *d);
    series
}
