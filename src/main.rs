mod correlation;
mod data;
mod report;
mod series;

use anyhow::Result;
use chrono::NaiveDate;
use clap::Parser;
use report::RegimeReport;
use series::{align, log_returns};

/// Mesure si Bitcoin se comporte plutôt comme un actif tech (corrélé au Nasdaq)
/// ou comme une valeur refuge (corrélé à l'or), et comment ça évolue dans le temps.
#[derive(Parser)]
#[command(name = "btc-regime", version, about)]
struct Cli {
    /// Jours d'historique à récupérer (l'API CoinGecko gratuite plafonne à 365)
    #[arg(long, default_value_t = 365)]
    days: u32,

    /// Taille de la fenêtre glissante, en jours, pour chaque point de corrélation
    #[arg(long, default_value_t = 90)]
    window: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    eprintln!("Récupération des cours BTC (CoinGecko)...");
    let btc = data::fetch_btc(cli.days)?;

    eprintln!("Récupération des cours Nasdaq (Yahoo Finance)...");
    let nasdaq = data::fetch_yahoo("%5EIXIC", "1y")?;

    eprintln!("Récupération des cours Or (Yahoo Finance)...");
    let gold = data::fetch_yahoo("GC=F", "1y")?;

    // BTC/Nasdaq et BTC/Or sont alignés séparément car les calendriers de cotation
    // diffèrent (le crypto trade le week-end, pas les marchés traditionnels).
    let btc_nasdaq = align(&btc, &nasdaq);
    let btc_gold = align(&btc, &gold);

    let (nasdaq_dates, nasdaq_corr) = rolling_series(&btc_nasdaq, cli.window);
    let (_, gold_corr) = rolling_series(&btc_gold, cli.window);

    let len = nasdaq_corr.len().min(gold_corr.len());
    if len == 0 {
        anyhow::bail!(
            "pas assez de jours communs entre les séries pour une fenêtre de {}j",
            cli.window
        );
    }
    let dates = nasdaq_dates[nasdaq_dates.len() - len..].to_vec();
    let nasdaq_corr = nasdaq_corr[nasdaq_corr.len() - len..].to_vec();
    let gold_corr = gold_corr[gold_corr.len() - len..].to_vec();

    RegimeReport {
        dates: &dates,
        nasdaq_corr: &nasdaq_corr,
        gold_corr: &gold_corr,
        window: cli.window,
    }
    .print();

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
