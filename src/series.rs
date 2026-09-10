use chrono::NaiveDate;
use std::collections::BTreeMap;

pub type Series = Vec<(NaiveDate, f64)>;

/// Inner-joins two price series on date, sorted ascending by date.
pub fn align(a: &Series, b: &Series) -> Vec<(NaiveDate, f64, f64)> {
    let map_b: BTreeMap<NaiveDate, f64> = b.iter().cloned().collect();
    let mut out: Vec<_> = a
        .iter()
        .filter_map(|(date, va)| map_b.get(date).map(|vb| (*date, *va, *vb)))
        .collect();
    out.sort_by_key(|(d, _, _)| *d);
    out
}

/// Daily log returns from a price series (n prices -> n-1 returns).
pub fn log_returns(prices: &[f64]) -> Vec<f64> {
    prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
    }

    #[test]
    fn align_keeps_only_shared_dates_sorted() {
        let a = vec![(d(3), 1.0), (d(1), 2.0), (d(2), 3.0)];
        let b = vec![(d(1), 10.0), (d(3), 30.0)];
        let joined = align(&a, &b);
        assert_eq!(joined, vec![(d(1), 2.0, 10.0), (d(3), 1.0, 30.0)]);
    }

    #[test]
    fn log_returns_length_and_sign() {
        let prices = [100.0, 110.0, 99.0];
        let r = log_returns(&prices);
        assert_eq!(r.len(), 2);
        assert!(r[0] > 0.0);
        assert!(r[1] < 0.0);
    }
}
