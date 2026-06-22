use crate::CategoryScore;

const THEORETICAL_RAGE_TABLE: &[RagePairing] = &[
    RagePairing::new(575, 579, 8.0),
    RagePairing::new(579, 575, 8.0),
    RagePairing::new(540, 575, 7.5),
    RagePairing::new(540, 459, 8.5),
    RagePairing::new(540, 576, 8.0),
    RagePairing::new(540, 115, 8.3),
    RagePairing::new(540, 578, 8.5),
    RagePairing::new(540, 579, 8.0),
    RagePairing::new(182, 540, 6.5),
    RagePairing::new(182, 148, 6.3),
    RagePairing::new(182, 138, 6.5),
    RagePairing::new(182, 192, 6.5),
    RagePairing::new(148, 540, 6.8),
    RagePairing::new(148, 192, 6.8),
    RagePairing::new(138, 148, 6.8),
    RagePairing::new(138, 540, 7.3),
    RagePairing::new(138, 115, 6.8),
    RagePairing::new(103, 575, 8.3),
    RagePairing::new(103, 578, 8.7),
    RagePairing::new(103, 459, 8.7),
    RagePairing::new(509, 179, 4.2),
    RagePairing::new(509, 6, 4.2),
    RagePairing::new(509, 195, 4.2),
    RagePairing::new(611, 179, 7.0),
    RagePairing::new(611, 6, 7.2),
    RagePairing::new(611, 195, 7.3),
    RagePairing::new(187, 611, 7.3),
    RagePairing::new(611, 187, 7.2),
    RagePairing::new(187, 179, 7.2),
    RagePairing::new(179, 187, 7.0),
    RagePairing::new(179, 459, 7.2),
    RagePairing::new(186, 6, 7.3),
    RagePairing::new(187, 186, 7.5),
    RagePairing::new(195, 6, 7.3),
    RagePairing::new(545, 185, 7.0),
    RagePairing::new(545, 194, 8.5),
    RagePairing::new(595, 185, 6.2),
    RagePairing::new(595, 545, 6.6),
    RagePairing::new(595, 194, 7.0),
    RagePairing::new(595, 596, 6.3),
    RagePairing::new(185, 98, 8.0),
    RagePairing::new(185, 459, 8.0),
    RagePairing::new(185, 194, 8.0),
    RagePairing::new(194, 185, 8.0),
    RagePairing::new(198, 140, 7.3),
    RagePairing::new(140, 198, 7.3),
    RagePairing::new(140, 185, 6.2),
    RagePairing::new(190, 189, 9.5),
    RagePairing::new(189, 190, 9.5),
    RagePairing::new(175, 189, 8.5),
    RagePairing::new(461, 460, 8.0),
];

const THEORETICAL_ASSIST_TABLE: &[AssistCommander] = &[
    AssistCommander::new(575, 8.0),
    AssistCommander::new(99, 50.0),
    AssistCommander::new(138, 7.0),
    AssistCommander::new(98, 39.0),
    AssistCommander::new(546, 7.0),
    AssistCommander::new(611, 10.0),
    AssistCommander::new(540, 2.0),
    AssistCommander::new(186, 0.0),
    AssistCommander::new(103, 2.0),
    AssistCommander::new(545, 3.0),
    AssistCommander::new(192, 24.0),
    AssistCommander::new(146, 12.0),
    AssistCommander::new(130, 0.0),
    AssistCommander::new(578, 2.0),
    AssistCommander::new(190, 10.0),
    AssistCommander::new(579, 6.0),
    AssistCommander::new(189, 0.0),
    AssistCommander::new(108, 18.0),
    AssistCommander::new(576, 5.0),
    AssistCommander::new(187, 41.0),
    AssistCommander::new(182, 5.0),
    AssistCommander::new(596, 0.0),
    AssistCommander::new(148, 11.0),
    AssistCommander::new(461, 0.0),
    AssistCommander::new(185, 16.0),
    AssistCommander::new(175, 1.0),
    AssistCommander::new(565, 0.0),
    AssistCommander::new(65, 0.0),
    AssistCommander::new(9, 2.0),
    AssistCommander::new(459, 40.0),
    AssistCommander::new(509, 5.0),
    AssistCommander::new(198, 4.0),
    AssistCommander::new(162, 7.0),
    AssistCommander::new(197, 4.0),
    AssistCommander::new(140, 23.0),
    AssistCommander::new(195, 16.0),
    AssistCommander::new(460, 0.0),
    AssistCommander::new(595, 0.0),
    AssistCommander::new(100, 13.0),
    AssistCommander::new(125, 52.0),
    AssistCommander::new(616, 46.0),
    AssistCommander::new(115, 21.0),
    AssistCommander::new(194, 0.0),
    AssistCommander::new(6, 0.0),
    AssistCommander::new(179, 30.0),
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
struct RagePairing {
    primary_commander_id: u32,
    secondary_commander_id: u32,
    avg_cycle: f64,
}

impl RagePairing {
    const fn new(primary_commander_id: u32, secondary_commander_id: u32, avg_cycle: f64) -> Self {
        Self { primary_commander_id, secondary_commander_id, avg_cycle }
    }

    const fn matches(self, primary_commander_id: u32, secondary_commander_id: u32) -> bool {
        self.primary_commander_id == primary_commander_id
            && self.secondary_commander_id == secondary_commander_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AssistCommander {
    commander_id: u32,
    assist_raw: f64,
}

impl AssistCommander {
    const fn new(commander_id: u32, assist_raw: f64) -> Self {
        Self { commander_id, assist_raw }
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
) -> TheoreticalValues {
    let avg_cycle = THEORETICAL_RAGE_TABLE
        .iter()
        .find(|pairing| pairing.matches(primary_commander_id, secondary_commander_id))
        .map_or(0.0, |pairing| pairing.avg_cycle);
    let assist_raw = assist_raw_for_commander(primary_commander_id)
        + assist_raw_for_commander(secondary_commander_id);

    TheoreticalValues::new(avg_cycle, assist_raw)
}

fn rage_score(avg_cycle: f64) -> f64 {
    let scaled = ((10.0 - avg_cycle) / 6.0).clamp(0.0, 1.0);
    10.0 * scaled.powf(0.55)
}

fn assist_score(assist_raw: f64) -> f64 {
    let scaled = (assist_raw / 100.0).clamp(0.0, 1.0);
    10.0 * scaled.powf(0.55)
}

fn assist_raw_for_commander(commander_id: u32) -> f64 {
    THEORETICAL_ASSIST_TABLE
        .iter()
        .find(|commander| commander.commander_id == commander_id)
        .map_or(0.0, |commander| commander.assist_raw)
}
