mod correlation;
mod data;
mod pulse;
mod report;
mod series;

use anyhow::Result;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use pulse::{AssetPulse, PulseReport};
use report::RegimeReport;
use series::{align, log_returns};

/// Outils de lecture de marché BTC inspirés d'une newsletter macro crypto :
/// régime de corrélation (tech vs valeur refuge) et pouls du marché des futures.
#[derive(Parser)]
#[command(name = "btc-regime", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Corrélation glissante BTC vs Nasdaq/Or (recrée le "Chart of the week")
    Regime {
        /// Jours d'historique à récupérer (l'API CoinGecko gratuite plafonne à 365)
        #[arg(long, default_value_t = 365)]
        days: u32,

        /// Taille de la fenêtre glissante, en jours, pour chaque point de corrélation
        #[arg(long, default_value_t = 90)]
        window: usize,
    },
    /// Funding rate & open interest BTC/ETH en direct (Binance Futures, recrée la section "Crypto")
    Pulse {
        /// Fenêtre de comparaison pour la variation d'open interest, en jours
        #[arg(long, default_value_t = 7)]
        days: u32,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Regime { days, window } => run_regime(days, window),
        Command::Pulse { days } => run_pulse(days),
    }
}

fn run_regime(days: u32, window: usize) -> Result<()> {
    eprintln!("Récupération des cours BTC (CoinGecko)...");
    let btc = data::fetch_btc(days)?;

    eprintln!("Récupération des cours Nasdaq (Yahoo Finance)...");
    let nasdaq = data::fetch_yahoo("%5EIXIC", "1y")?;

    eprintln!("Récupération des cours Or (Yahoo Finance)...");
    let gold = data::fetch_yahoo("GC=F", "1y")?;

    // BTC/Nasdaq et BTC/Or sont alignés séparément car les calendriers de cotation
    // diffèrent (le crypto trade le week-end, pas les marchés traditionnels).
    let btc_nasdaq = align(&btc, &nasdaq);
    let btc_gold = align(&btc, &gold);

    let (nasdaq_dates, nasdaq_corr) = rolling_series(&btc_nasdaq, window);
    let (_, gold_corr) = rolling_series(&btc_gold, window);

    let len = nasdaq_corr.len().min(gold_corr.len());
    if len == 0 {
        anyhow::bail!(
            "pas assez de jours communs entre les séries pour une fenêtre de {}j",
            window
        );
    }
    let dates = nasdaq_dates[nasdaq_dates.len() - len..].to_vec();
    let nasdaq_corr = nasdaq_corr[nasdaq_corr.len() - len..].to_vec();
    let gold_corr = gold_corr[gold_corr.len() - len..].to_vec();

    RegimeReport {
        dates: &dates,
        nasdaq_corr: &nasdaq_corr,
        gold_corr: &gold_corr,
        window,
    }
    .print();

    Ok(())
}

fn run_pulse(days: u32) -> Result<()> {
    let mut assets = Vec::new();
    for (binance_symbol, label) in [("BTCUSDT", "BTC"), ("ETHUSDT", "ETH")] {
        eprintln!("Récupération des données {label} (Binance Futures)...");
        let snapshot = pulse::fetch_premium_index(binance_symbol)?;
        let oi_history = pulse::fetch_oi_history(binance_symbol, days)?;

        let (oi_first, oi_last) = match (oi_history.first(), oi_history.last()) {
            (Some(first), Some(last)) => (*first, *last),
            _ => anyhow::bail!("historique d'open interest vide pour {binance_symbol}"),
        };
        let oi_change_pct = pulse::pct_change(oi_first, oi_last);
        let froth = pulse::classify(snapshot.funding_rate, oi_change_pct);

        assets.push(AssetPulse {
            symbol: label,
            mark_price: snapshot.mark_price,
            funding_rate: snapshot.funding_rate,
            oi_now: oi_last,
            oi_change_pct,
            froth,
        });
    }

    PulseReport { assets }.print();
    Ok(())
}

/// Convertit une série alignée (date, prix BTC, prix autre actif) en corrélations
/// glissantes calculées sur les rendements journaliers log, chacune datée par le
/// dernier jour de sa fenêtre.
fn rolling_series(aligned: &[(NaiveDate, f64, f64)], window: usize) -> (Vec<NaiveDate>, Vec<f64>) {
    let dates: Vec<_> = aligned.iter().map(|(d, _, _)| *d).collect();
    let btc_prices: Vec<_> = aligned.iter().map(|(_, b, _)| *b).collect();
    let other_prices: Vec<_> = aligned.iter().map(|(_, _, o)| *o).collect();

    let btc_returns = log_returns(&btc_prices);
    let other_returns = log_returns(&other_prices);

    correlation::rolling_correlation(&btc_returns, &other_returns, window)
        .into_iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|value| (dates[i + window], value)))
        .unzip()
}
