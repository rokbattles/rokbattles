//! Scores sample coverage independently of battle performance.

use serde::Serialize;

const EFFECTIVE_GOVERNOR_TARGET: f64 = 200.0;
const BATTLE_TARGET: f64 = 5_000.0;
const GOVERNOR_WEIGHT: f64 = 0.70;
const BATTLE_WEIGHT: f64 = 0.30;

/// A sample-coverage score based on battle count and governor concentration.
///
/// This is a heuristic on a 0–10 scale, not a statistical confidence interval
/// or a probability that a performance score is correct. It is calculated
/// separately from [`crate::DrastcModel`] and does not alter category weights.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcConfidence {
    /// Combined governor-coverage and battle-count score on a 0–10 scale.
    pub score: f64,
    /// Number of distinct identified governors, as supplied by the caller.
    pub unique_governors: u64,
    /// Effective governor count, `total_battles^2 / sum(governor_battles^2)`.
    /// Equal contributions give the actual governor count; concentrated
    /// contributions reduce it when the supplied totals are consistent.
    pub effective_governors: f64,
}

impl DrastcConfidence {
    /// Calculates sample coverage from a governor battle-count distribution.
    ///
    /// Supply `total_battles = sum(n_i)`, `unique_governors` as the number of
    /// contributing governors, and `governor_battles_squared_sum = sum(n_i^2)`,
    /// where `n_i` is one governor's battle count. These inputs must describe
    /// the same sample; their consistency is not checked.
    ///
    /// Effective governors and battle count each produce a factor
    /// `1 - 10^(-value / target)`, using targets of 200 governors and 5,000
    /// battles. The result is `10 * governor_factor^0.7 * battle_factor^0.3`.
    /// Each factor reaches 0.9 at its target and approaches 1 as coverage grows.
    ///
    /// Returns zero score and effective governors if either count is zero or
    /// the squared sum is non-finite or non-positive. `unique_governors` is
    /// always returned unchanged and is not otherwise used in the formula.
    ///
    /// # Examples
    ///
    /// Two governors contributing 50 battles each have an effective count of two:
    ///
    /// ```
    /// use rokbattles_drastc::DrastcConfidence;
    ///
    /// let confidence = DrastcConfidence::from_governor_distribution(
    ///     100, 2, 50.0_f64.powi(2) + 50.0_f64.powi(2),
    /// );
    /// assert_eq!(confidence.effective_governors, 2.0);
    /// ```
    pub fn from_governor_distribution(
        total_battles: u64,
        unique_governors: u64,
        governor_battles_squared_sum: f64,
    ) -> Self {
        if total_battles == 0
            || unique_governors == 0
            || !governor_battles_squared_sum.is_finite()
            || governor_battles_squared_sum <= 0.0
        {
            return Self { score: 0.0, unique_governors, effective_governors: 0.0 };
        }

        let total_battles_f64 = total_battles as f64;
        // Inverting the sum of squared contribution shares discounts samples
        // dominated by a few governors, even when many IDs appear in the data.
        let effective_governors =
            total_battles_f64 * total_battles_f64 / governor_battles_squared_sum;
        let governor_factor = exponential_factor(effective_governors, EFFECTIVE_GOVERNOR_TARGET);
        let battle_factor = exponential_factor(total_battles_f64, BATTLE_TARGET);
        // The geometric combination requires both coverage factors to contribute.
        let score =
            10.0 * governor_factor.powf(GOVERNOR_WEIGHT) * battle_factor.powf(BATTLE_WEIGHT);

        Self { score: score.clamp(0.0, 10.0), unique_governors, effective_governors }
    }
}

fn exponential_factor(value: f64, target: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 || !target.is_finite() || target <= 0.0 {
        0.0
    } else {
        (1.0 - 10.0_f64.powf(-value / target)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.005, "expected {expected}, got {actual}");
    }

    #[test]
    fn effective_governors_equal_unique_governors_for_even_contributions() {
        let confidence = DrastcConfidence::from_governor_distribution(100, 2, 5_000.0);

        assert_approx_eq(confidence.effective_governors, 2.0);
    }

    #[test]
    fn confidence_matches_sun_tzu_prime_ivar_sample() {
        let confidence = DrastcConfidence::from_governor_distribution(111_512, 816, 437_627_902.0);

        assert_approx_eq(confidence.score, 4.09);
    }

    #[test]
    fn confidence_matches_representative_pairing_targets() {
        let cases = [
            (2_551_278, 22_153, 230.74, 9.50),
            (105_429, 2_136, 51.60, 5.70),
            (536_682, 10_177, 65.43, 6.40),
            (805_817, 10_328, 78.89, 6.97),
            (111_512, 816, 28.41, 4.09),
            (1_331_076, 37_021, 230.50, 9.50),
        ];

        for (battles, unique_governors, effective_governors, expected_score) in cases {
            let battles_f64 = battles as f64;
            let squared_sum = battles_f64 * battles_f64 / effective_governors;
            let confidence = DrastcConfidence::from_governor_distribution(
                battles,
                unique_governors,
                squared_sum,
            );

            assert!(
                (confidence.score - expected_score).abs() < 0.01,
                "expected {expected_score}, got {}",
                confidence.score
            );
        }
    }

    #[test]
    fn confidence_is_zero_without_identified_governors() {
        let confidence = DrastcConfidence::from_governor_distribution(100, 0, 10_000.0);

        assert_eq!(confidence.score, 0.0);
    }

    #[test]
    fn exponential_factor_is_ninety_percent_at_target() {
        assert_approx_eq(exponential_factor(200.0, 200.0), 0.9);
    }
}
