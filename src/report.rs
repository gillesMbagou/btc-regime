use chrono::NaiveDate;
use colored::{ColoredString, Colorize};
use textplots::{Chart, Plot, Shape};

pub struct RegimeReport<'a> {
    pub dates: &'a [NaiveDate],
    pub nasdaq_corr: &'a [f64],
    pub gold_corr: &'a [f64],
    pub window: usize,
}

impl RegimeReport<'_> {
    pub fn print(&self) {
        self.print_header();
        self.print_summary();
        self.print_chart();
        self.print_crossover();
    }

    fn print_header(&self) {
        println!(
            "\n{}",
            format!(
                "Régime de corrélation BTC — fenêtre glissante {}j (rendements journaliers)",
                self.window
            )
            .bold()
        );
        println!("{}", "─".repeat(70).dimmed());
    }

    fn print_summary(&self) {
        let last_n = self.nasdaq_corr.last().copied().unwrap_or(0.0);
        let last_g = self.gold_corr.last().copied().unwrap_or(0.0);
        let max_n = self.nasdaq_corr.iter().cloned().fold(f64::MIN, f64::max);
        let max_g = self.gold_corr.iter().cloned().fold(f64::MIN, f64::max);

        println!("  {:<32} {}", "BTC vs Nasdaq (actuel):", fmt_corr(last_n));
        println!("  {:<32} {}", "BTC vs Or (actuel):", fmt_corr(last_g));
        println!(
            "  {:<32} {}",
            "BTC vs Nasdaq (pic période):",
            fmt_corr(max_n)
        );
        println!("  {:<32} {}", "BTC vs Or (pic période):", fmt_corr(max_g));
    }

    fn print_chart(&self) {
        let to_points = |series: &[f64]| -> Vec<(f32, f32)> {
            series
                .iter()
                .enumerate()
                .map(|(i, v)| (i as f32, *v as f32))
                .collect()
        };
        let nasdaq_points = to_points(self.nasdaq_corr);
        let gold_points = to_points(self.gold_corr);
        let max_x = (self.nasdaq_corr.len().max(2) - 1) as f32;

        println!("\n  {} = Nasdaq   {} = Or", "─".cyan(), "─".yellow());
        Chart::new(140, 44, 0.0, max_x)
            .lineplot(&Shape::Lines(&nasdaq_points))
            .lineplot(&Shape::Lines(&gold_points))
            .display();
        if let (Some(first), Some(last)) = (self.dates.first(), self.dates.last()) {
            println!(
                "  {} {} {}",
                first.format("%d %b %Y"),
                "→".dimmed(),
                last.format("%d %b %Y")
            );
        }
    }

    fn print_crossover(&self) {
        let n = self
            .dates
            .len()
            .min(self.nasdaq_corr.len())
            .min(self.gold_corr.len());

        let crossover = (1..n).rev().find(|&i| {
            self.gold_corr[i - 1] <= self.nasdaq_corr[i - 1]
                && self.gold_corr[i] > self.nasdaq_corr[i]
        });

        match crossover {
            Some(i) => println!(
                "\n  {} {}",
                "Dernier croisement Or > Nasdaq :".bold(),
                self.dates[i].format("%d %b %Y").to_string().green()
            ),
            None => println!("\n  Aucun croisement Or > Nasdaq sur la période analysée."),
        }
    }
}

fn fmt_corr(v: f64) -> ColoredString {
    let pct = format!("{:+.0}%", v * 100.0);
    if v > 0.15 {
        pct.green()
    } else if v < -0.15 {
        pct.red()
    } else {
        pct.normal()
    }
}
