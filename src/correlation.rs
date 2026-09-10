/// Pearson correlation coefficient between two equal-length slices.
/// Returns None if the inputs are empty, mismatched, or either has zero variance.
pub fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n == 0 || n != y.len() {
        return None;
    }
    let n_f = n as f64;
    let mean_x = x.iter().sum::<f64>() / n_f;
    let mean_y = y.iter().sum::<f64>() / n_f;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    if var_x == 0.0 || var_y == 0.0 {
        return None;
    }
    Some(cov / (var_x.sqrt() * var_y.sqrt()))
}

/// Pearson correlation over each sliding window of the given size.
/// Output has `x.len() - window + 1` entries, one per window position.
pub fn rolling_correlation(x: &[f64], y: &[f64], window: usize) -> Vec<Option<f64>> {
    if window == 0 || x.len() != y.len() || x.len() < window {
        return Vec::new();
    }
    (0..=x.len() - window)
        .map(|start| pearson(&x[start..start + window], &y[start..start + window]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_positive_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((pearson(&x, &y).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn perfect_negative_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [10.0, 8.0, 6.0, 4.0, 2.0];
        assert!((pearson(&x, &y).unwrap() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn zero_variance_is_undefined() {
        let x = [1.0, 1.0, 1.0];
        let y = [1.0, 2.0, 3.0];
        assert_eq!(pearson(&x, &y), None);
    }

    #[test]
    fn mismatched_lengths_are_none() {
        assert_eq!(pearson(&[1.0, 2.0], &[1.0]), None);
    }

    #[test]
    fn rolling_window_output_length() {
        let x = [1.0, 2.0, 1.5, 3.0, 2.5, 4.0, 3.5, 5.0, 4.5, 6.0];
        let y = [2.0, 3.0, 2.5, 4.0, 3.5, 5.0, 4.5, 6.0, 5.5, 7.0];
        let r = rolling_correlation(&x, &y, 4);
        assert_eq!(r.len(), x.len() - 4 + 1);
        assert!(r.iter().all(|v| v.is_some()));
    }

    #[test]
    fn window_larger_than_input_is_empty() {
        let x = [1.0, 2.0];
        let y = [1.0, 2.0];
        assert!(rolling_correlation(&x, &y, 5).is_empty());
    }
}
