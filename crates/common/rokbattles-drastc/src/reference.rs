//! Maps metrics onto reference bounds and applies optional score curves.

use serde::Serialize;

use crate::{CategoryScore, MIN_REFERENCE_RANGE};

/// Externally calculated bounds for the four battle-derived categories.
///
/// Each range must use the same metric and units as its corresponding category.
/// Rage and Assist use fixed bounds and do not read these ranges.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcReferenceRanges {
    /// Bounds for inflicted casualties per second.
    pub damage: ReferenceRange,
    /// Bounds for healing minus received casualties per second; may be negative.
    pub sustainability: ReferenceRange,
    /// Bounds for the sender-to-opponent kill-point ratio.
    pub trade: ReferenceRange,
    /// Bounds for the mean of the available win and positive-trade rates.
    pub consistency: ReferenceRange,
}

/// P10/P90 benchmark bounds and reference sample count for one metric.
///
/// Linear scoring maps `p10` to 0 and `p90` to 10, clamping values outside the
/// interval. Damage and Sustainability apply a power curve afterward. These
/// bounds locate a metric between two benchmarks; the score is not a percentile
/// rank within the original population.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRange {
    sample_count: usize,
    /// Lower benchmark, conventionally the population's 10th percentile.
    pub p10: f64,
    /// Upper benchmark, conventionally the population's 90th percentile.
    pub p90: f64,
}

impl ReferenceRange {
    /// Creates a range from a reference sample count and precomputed bounds.
    ///
    /// Supply finite bounds with `p10 <= p90`. This constructor stores its inputs
    /// unchanged; it does not sort samples, compute percentiles, or validate bounds.
    ///
    /// A zero sample count or non-finite metric scores zero. Otherwise, bounds
    /// within `1e-9` of each other yield a linear score of 5, regardless of the
    /// metric. Damage and Sustainability still apply their curve to that score.
    /// The sample count does not otherwise affect normalization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rokbattles_drastc::ReferenceRange;
    ///
    /// let damage = ReferenceRange::new(200, 1.2, 2.8);
    /// assert_eq!(damage.sample_count(), 200);
    /// assert_eq!(damage.p10, 1.2);
    /// ```
    pub const fn new(sample_count: usize, p10: f64, p90: f64) -> Self {
        Self { sample_count, p10, p90 }
    }

    /// Returns the reference population size supplied at construction.
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
            // Exponents below one raise interior scores while preserving 0 and 10.
            10.0 * (linear_score / 10.0).powf(exponent)
        };

        CategoryScore { value, p10: self.p10, p90: self.p90, score }
    }

    fn linear_score(self, value: f64) -> f64 {
        if self.sample_count == 0 || !value.is_finite() {
            0.0
        } else if (self.p90 - self.p10).abs() <= MIN_REFERENCE_RANGE {
            // A collapsed range has no usable spread; use its midpoint score.
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

    #[expect(clippy::cast_sign_loss, reason = "Clamped percentile ranks are nonnegative.")]
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
