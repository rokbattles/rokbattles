use serde::Serialize;

use crate::{CategoryScore, MIN_REFERENCE_RANGE};

/// Percentile reference ranges used by percentile-based DRASTC categories.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcReferenceRanges {
    /// Damage reference range.
    pub damage: ReferenceRange,
    /// Sustainability reference range.
    pub sustainability: ReferenceRange,
    /// Trade reference range.
    pub trade: ReferenceRange,
    /// Consistency reference range.
    pub consistency: ReferenceRange,
}

/// P10/P90 benchmark range for one DRASTC metric.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRange {
    sample_count: usize,
    /// P10 reference value.
    pub p10: f64,
    /// P90 reference value.
    pub p90: f64,
}

impl ReferenceRange {
    /// Create a reference range from precomputed values.
    pub const fn new(sample_count: usize, p10: f64, p90: f64) -> Self {
        Self { sample_count, p10, p90 }
    }

    /// Number of samples used for this range.
    pub const fn sample_count(self) -> usize {
        self.sample_count
    }

    pub(crate) fn score(self, value: f64) -> CategoryScore {
        CategoryScore { value, p10: self.p10, p90: self.p90, score: self.linear_score(value) }
    }

    pub(crate) fn score_curved(self, value: f64, exponent: f64) -> CategoryScore {
        let linear_score = self.linear_score(value);
        let score = if linear_score <= 0.0 || !exponent.is_finite() || exponent <= 0.0 {
            linear_score
        } else {
            10.0 * (linear_score / 10.0).powf(exponent)
        };

        CategoryScore { value, p10: self.p10, p90: self.p90, score }
    }

    fn linear_score(self, value: f64) -> f64 {
        if self.sample_count == 0 || !value.is_finite() {
            0.0
        } else if (self.p90 - self.p10).abs() <= MIN_REFERENCE_RANGE {
            5.0
        } else {
            (10.0 * ((value - self.p10) / (self.p90 - self.p10))).clamp(0.0, 10.0)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn percentile_interpolates_sorted_values() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(percentile(&values, 0.10), 1.4);
        assert_eq!(percentile(&values, 0.90), 4.6);
    }

    fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
        match sorted_values {
            [] => 0.0,
            [value] => *value,
            values => {
                let rank = percentile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
                let lower = rank.floor() as usize;
                let upper = rank.ceil() as usize;
                let lower_value = values.get(lower).copied().unwrap_or(0.0);
                let upper_value = values.get(upper).copied().unwrap_or(lower_value);
                lower_value + ((upper_value - lower_value) * (rank - lower as f64))
            }
        }
    }
}
