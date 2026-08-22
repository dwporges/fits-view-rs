use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum Scaling {
    LINEAR,
    LOGARITHMIC,
    SQUAREROOT,
    ASINH,
}

impl Scaling {
    pub fn scale(&self, values: &[f64], min: f64, max: f64) -> Option<Vec<f64>> {
        match self {
            Scaling::LINEAR => scale_linear(values, min, max),
            Scaling::LOGARITHMIC => scale_logarithmic(values, min, max),
            Scaling::SQUAREROOT => scale_squareroot(values, min, max),
            Scaling::ASINH => scale_asinh(values, min, max),
        }
    }
}

impl Default for Scaling {
    fn default() -> Self {
        Scaling::LINEAR
    }
}

impl fmt::Display for Scaling {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Scaling::LINEAR => write!(f, "Linear"),
            Scaling::LOGARITHMIC => write!(f, "Logarithmic"),
            Scaling::SQUAREROOT => write!(f, "Square Root"),
            Scaling::ASINH => write!(f, "Asinh"),
        }
    }
}

pub fn get_percentile_min_max(data: &[f64], lower_pct: f64, upper_pct: f64) -> Option<(f64, f64)> {
    if data.is_empty() {
        return None;
    }

    // following commented lines are an uneccesary optimisation that have been removed
    // let step = if data.len() > 1_000_000 { 100 } else { 1 };
    // let mut sorted: Vec<f64> = data.iter().step_by(step).copied().collect();

    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let lower_idx = ((sorted.len() as f64 * lower_pct).floor()) as usize;
    let upper_idx = ((sorted.len() as f64 * upper_pct).floor()) as usize;

    let min_val = sorted[lower_idx.clamp(0, sorted.len() - 1)];
    let max_val = sorted[upper_idx.clamp(0, sorted.len() - 1)];

    Some((min_val, max_val))
}

pub fn scale_linear(data: &[f64], min: f64, max: f64) -> Option<Vec<f64>> {
    let range = max - min;

    // if min >= max, return an empty black canvas
    if range <= 0.0 {
        return Some(vec![0.0; data.len()]);
    }

    Some(
        data.iter()
            .map(|&x| {
                let clamped = x.clamp(min, max);
                ((clamped - min) / range) * 255.0
            })
            .collect(),
    )
}

pub fn scale_logarithmic(data: &[f64], min: f64, max: f64) -> Option<Vec<f64>> {
    let range = max - min;

    if range <= 0.0 {
        return Some(vec![0.0; data.len()]);
    }

    let log_range = (range + 1.0).ln();

    Some(
        data.iter()
            .map(|&x| {
                let clamped = x.clamp(min, max);
                let normalized = (clamped - min + 1.0).ln() / log_range;
                normalized * 255.0
            })
            .collect(),
    )
}

pub fn scale_asinh(data: &[f64], min: f64, max: f64) -> Option<Vec<f64>> {
    let range = max - min;

    if range <= 0.0 {
        return Some(vec![0.0; data.len()]);
    }

    let asinh_range = range.asinh();

    Some(
        data.iter()
            .map(|&x| {
                let clamped = x.clamp(min, max);
                let normalized = (clamped - min).asinh() / asinh_range;
                normalized * 255.0
            })
            .collect(),
    )
}

pub fn scale_squareroot(data: &[f64], min: f64, max: f64) -> Option<Vec<f64>> {
    let range = max - min;

    if range <= 0.0 {
        return Some(vec![0.0; data.len()]);
    }

    let sqrt_range = range.sqrt();

    Some(
        data.iter()
            .map(|&x| {
                let clamped = x.clamp(min, max);
                let normalized = (clamped - min).sqrt() / sqrt_range;
                normalized * 255.0
            })
            .collect(),
    )
}
