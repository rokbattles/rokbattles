use crate::CategoryScore;

/// Static Rage entries used by a DRASTC model.
pub type RageTable = &'static [RagePairing];

/// Season of Conquest Rage values.
pub const SOC_RAGE_TABLE: RageTable = &[
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
    RagePairing::new(509, 195, 4.5),
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
    RagePairing::new(545, 576, 8.5),
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
    RagePairing::new(540, 616, 7.5),
    RagePairing::new(575, 616, 7.5),
    RagePairing::new(545, 616, 7.8),
    RagePairing::new(185, 616, 7.3),
    RagePairing::new(459, 616, 7.5),
];

/// Pre-Season of Conquest Rage values.
pub const PRESOC_RAGE_TABLE: RageTable = &[
    RagePairing::new(141, 6, 8.5),
    RagePairing::new(141, 99, 9.7),
    RagePairing::new(6, 65, 8.7),
    RagePairing::new(141, 100, 9.7),
    RagePairing::new(121, 6, 8.7),
    RagePairing::new(10, 180, 11.0),
    RagePairing::new(10, 3, 10.0),
    RagePairing::new(180, 130, 11.0),
    RagePairing::new(10, 6, 9.7),
    RagePairing::new(3, 6, 7.5),
    RagePairing::new(3, 99, 8.7),
    RagePairing::new(9, 618, 7.5),
    RagePairing::new(9, 7, 8.3),
    RagePairing::new(9, 62, 8.3),
    RagePairing::new(9, 6, 7.5),
    RagePairing::new(64, 6, 8.0),
    RagePairing::new(64, 99, 8.7),
    RagePairing::new(64, 618, 8.0),
];

const ASSIST_TABLE: &[AssistCommander] = &[
    AssistCommander::new(575, 8.0),
    AssistCommander::new(99, 50.2),
    AssistCommander::new(138, 7.02),
    AssistCommander::new(98, 39.2),
    AssistCommander::new(546, 6.666666667),
    AssistCommander::new(611, 10.05),
    AssistCommander::new(540, 1.8),
    AssistCommander::new(186, 0.0),
    AssistCommander::new(103, 2.466666667),
    AssistCommander::new(62, 12.5),
    AssistCommander::new(545, 3.0),
    AssistCommander::new(192, 24.0),
    AssistCommander::new(146, 11.5),
    AssistCommander::new(7, 0.0),
    AssistCommander::new(130, 0.0),
    AssistCommander::new(121, 1.56),
    AssistCommander::new(578, 2.0),
    AssistCommander::new(190, 9.72),
    AssistCommander::new(579, 6.24),
    AssistCommander::new(189, 0.0),
    AssistCommander::new(108, 18.0),
    AssistCommander::new(576, 5.0),
    AssistCommander::new(187, 41.08),
    AssistCommander::new(182, 4.615384615),
    AssistCommander::new(596, 0.0),
    AssistCommander::new(148, 10.8),
    AssistCommander::new(461, 0.0),
    AssistCommander::new(185, 16.0),
    AssistCommander::new(175, 0.72),
    AssistCommander::new(10, 0.0),
    AssistCommander::new(565, 0.0),
    AssistCommander::new(65, 0.0),
    AssistCommander::new(9, 1.8),
    AssistCommander::new(459, 40.0),
    AssistCommander::new(180, 1.24875),
    AssistCommander::new(509, 4.5),
    AssistCommander::new(198, 4.02),
    AssistCommander::new(64, 0.0),
    AssistCommander::new(162, 6.5),
    AssistCommander::new(197, 4.0),
    AssistCommander::new(140, 22.905),
    AssistCommander::new(195, 15.56),
    AssistCommander::new(460, 0.0),
    AssistCommander::new(595, 0.0),
    AssistCommander::new(3, 0.0),
    AssistCommander::new(141, 0.73125),
    AssistCommander::new(100, 3.9),
    AssistCommander::new(125, 52.0),
    AssistCommander::new(616, 45.6),
    AssistCommander::new(115, 20.6),
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

/// One ordered commander pairing and its theoretical average Rage cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RagePairing {
    /// Primary commander ID.
    pub primary_commander_id: u32,
    /// Secondary commander ID.
    pub secondary_commander_id: u32,
    /// Theoretical average skill cycle for this ordered pairing.
    pub avg_cycle: f64,
}

impl RagePairing {
    /// Create a static Rage-table entry.
    pub const fn new(
        primary_commander_id: u32,
        secondary_commander_id: u32,
        avg_cycle: f64,
    ) -> Self {
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
    rage_table: RageTable,
    primary_commander_id: u32,
    secondary_commander_id: u32,
) -> TheoreticalValues {
    let avg_cycle = rage_table
        .iter()
        .find(|pairing| pairing.matches(primary_commander_id, secondary_commander_id))
        .map_or(0.0, |pairing| pairing.avg_cycle);
    let assist_raw = assist_raw_for_commander(primary_commander_id)
        + assist_raw_for_commander(secondary_commander_id);

    TheoreticalValues::new(avg_cycle, assist_raw)
}

pub(crate) fn is_supported_pairing(
    rage_table: RageTable,
    primary_commander_id: u32,
    secondary_commander_id: u32,
) -> bool {
    rage_table.iter().any(|pairing| pairing.matches(primary_commander_id, secondary_commander_id))
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
    ASSIST_TABLE
        .iter()
        .find(|commander| commander.commander_id == commander_id)
        .map_or(0.0, |commander| commander.assist_raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soc_rage_matches_all_static_values() {
        let actual = SOC_RAGE_TABLE
            .iter()
            .map(|pairing| {
                (pairing.primary_commander_id, pairing.secondary_commander_id, pairing.avg_cycle)
            })
            .collect::<Vec<_>>();
        let expected = vec![
            (575, 579, 8.0),
            (579, 575, 8.0),
            (540, 575, 7.5),
            (540, 459, 8.5),
            (540, 576, 8.0),
            (540, 115, 8.3),
            (540, 578, 8.5),
            (540, 579, 8.0),
            (182, 540, 6.5),
            (182, 148, 6.3),
            (182, 138, 6.5),
            (182, 192, 6.5),
            (148, 540, 6.8),
            (148, 192, 6.8),
            (138, 148, 6.8),
            (138, 540, 7.3),
            (138, 115, 6.8),
            (103, 575, 8.3),
            (103, 578, 8.7),
            (103, 459, 8.7),
            (509, 179, 4.2),
            (509, 6, 4.2),
            (509, 195, 4.5),
            (611, 179, 7.0),
            (611, 6, 7.2),
            (611, 195, 7.3),
            (187, 611, 7.3),
            (611, 187, 7.2),
            (187, 179, 7.2),
            (179, 187, 7.0),
            (179, 459, 7.2),
            (186, 6, 7.3),
            (187, 186, 7.5),
            (195, 6, 7.3),
            (545, 185, 7.0),
            (545, 194, 8.5),
            (545, 576, 8.5),
            (595, 185, 6.2),
            (595, 545, 6.6),
            (595, 194, 7.0),
            (595, 596, 6.3),
            (185, 98, 8.0),
            (185, 459, 8.0),
            (185, 194, 8.0),
            (194, 185, 8.0),
            (198, 140, 7.3),
            (140, 198, 7.3),
            (140, 185, 6.2),
            (190, 189, 9.5),
            (189, 190, 9.5),
            (175, 189, 8.5),
            (461, 460, 8.0),
            (540, 616, 7.5),
            (575, 616, 7.5),
            (545, 616, 7.8),
            (185, 616, 7.3),
            (459, 616, 7.5),
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn presoc_rage_matches_all_static_values() {
        let actual = PRESOC_RAGE_TABLE
            .iter()
            .map(|pairing| {
                (pairing.primary_commander_id, pairing.secondary_commander_id, pairing.avg_cycle)
            })
            .collect::<Vec<_>>();
        let expected = vec![
            (141, 6, 8.5),
            (141, 99, 9.7),
            (6, 65, 8.7),
            (141, 100, 9.7),
            (121, 6, 8.7),
            (10, 180, 11.0),
            (10, 3, 10.0),
            (180, 130, 11.0),
            (10, 6, 9.7),
            (3, 6, 7.5),
            (3, 99, 8.7),
            (9, 618, 7.5),
            (9, 7, 8.3),
            (9, 62, 8.3),
            (9, 6, 7.5),
            (64, 6, 8.0),
            (64, 99, 8.7),
            (64, 618, 8.0),
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn assist_matches_all_static_values() {
        let actual = ASSIST_TABLE
            .iter()
            .map(|commander| (commander.commander_id, commander.assist_raw))
            .collect::<Vec<_>>();
        let expected = vec![
            (575, 8.0),
            (99, 50.2),
            (138, 7.02),
            (98, 39.2),
            (546, 6.666666667),
            (611, 10.05),
            (540, 1.8),
            (186, 0.0),
            (103, 2.466666667),
            (62, 12.5),
            (545, 3.0),
            (192, 24.0),
            (146, 11.5),
            (7, 0.0),
            (130, 0.0),
            (121, 1.56),
            (578, 2.0),
            (190, 9.72),
            (579, 6.24),
            (189, 0.0),
            (108, 18.0),
            (576, 5.0),
            (187, 41.08),
            (182, 4.615384615),
            (596, 0.0),
            (148, 10.8),
            (461, 0.0),
            (185, 16.0),
            (175, 0.72),
            (10, 0.0),
            (565, 0.0),
            (65, 0.0),
            (9, 1.8),
            (459, 40.0),
            (180, 1.24875),
            (509, 4.5),
            (198, 4.02),
            (64, 0.0),
            (162, 6.5),
            (197, 4.0),
            (140, 22.905),
            (195, 15.56),
            (460, 0.0),
            (595, 0.0),
            (3, 0.0),
            (141, 0.73125),
            (100, 3.9),
            (125, 52.0),
            (616, 45.6),
            (115, 20.6),
            (194, 0.0),
            (6, 0.0),
            (179, 30.0),
        ];

        assert_eq!(actual, expected);
    }
}
