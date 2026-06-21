use crate::CategoryScore;

const THEORETICAL_TABLE: &[TheoreticalPairing] = &[
    TheoreticalPairing::new(579, 575, 8.0, 22.0), // Gang Gamchan / Achilles
    TheoreticalPairing::new(509, 179, 4.2, 30.0), // Qin Shi Huang / Zhuge Liang
    TheoreticalPairing::new(179, 187, 7.0, 71.0), // Zhuge Liang / Hermann Prime
];

/// Static Rage/Assist inputs for a pairing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TheoreticalValues {
    /// Theoretical average skill cycle.
    pub avg_cycle: f64,
    /// Raw assist/support value on a 0-100 scale.
    pub assist_raw: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TheoreticalPairing {
    primary_commander_id: u32,
    secondary_commander_id: u32,
    values: TheoreticalValues,
}

impl TheoreticalPairing {
    const fn new(
        primary_commander_id: u32,
        secondary_commander_id: u32,
        avg_cycle: f64,
        assist_raw: f64,
    ) -> Self {
        Self {
            primary_commander_id,
            secondary_commander_id,
            values: TheoreticalValues::new(avg_cycle, assist_raw),
        }
    }

    const fn matches(self, primary_commander_id: u32, secondary_commander_id: u32) -> bool {
        self.primary_commander_id == primary_commander_id
            && self.secondary_commander_id == secondary_commander_id
    }
}

impl TheoreticalValues {
    /// Create theoretical Rage/Assist inputs.
    pub const fn new(avg_cycle: f64, assist_raw: f64) -> Self {
        Self { avg_cycle, assist_raw }
    }

    pub(crate) fn rage_score(self) -> CategoryScore {
        if self.avg_cycle <= 0.0 || !self.avg_cycle.is_finite() {
            return CategoryScore::fixed_zero();
        }

        CategoryScore {
            value: self.avg_cycle,
            p10: 10.0,
            p90: 4.0,
            score: rage_score(self.avg_cycle),
        }
    }

    pub(crate) fn assist_score(self) -> CategoryScore {
        if self.assist_raw <= 0.0 || !self.assist_raw.is_finite() {
            return CategoryScore::fixed_zero();
        }

        CategoryScore {
            value: self.assist_raw,
            p10: 0.0,
            p90: 100.0,
            score: assist_score(self.assist_raw),
        }
    }
}

pub(crate) fn theoretical_for_pairing(
    primary_commander_id: u32,
    secondary_commander_id: u32,
) -> Option<TheoreticalValues> {
    THEORETICAL_TABLE
        .iter()
        .find(|pairing| pairing.matches(primary_commander_id, secondary_commander_id))
        .map(|pairing| pairing.values)
}

fn rage_score(avg_cycle: f64) -> f64 {
    let scaled = ((10.0 - avg_cycle) / 6.0).clamp(0.0, 1.0);
    10.0 * scaled.powf(0.55)
}

fn assist_score(assist_raw: f64) -> f64 {
    let scaled = (assist_raw / 100.0).clamp(0.0, 1.0);
    10.0 * scaled.powf(0.55)
}
