//! Mail-type helpers shared across processor modules.

use std::fmt;

/// Supported mail categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailType {
    Battle,
    DuelBattle2,
    BarCanyonKillBoss,
}

impl MailType {
    /// Parse a supported mail type from the decoded `type` field.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "Battle" => Some(Self::Battle),
            "DuelBattle2" => Some(Self::DuelBattle2),
            "BarCanyonKillBoss" => Some(Self::BarCanyonKillBoss),
            _ => None,
        }
    }

    /// Return the MongoDB collection name for this mail type.
    pub fn collection_name(self) -> &'static str {
        match self {
            Self::Battle => "mails_battle",
            Self::DuelBattle2 => "mails_duelbattle2",
            Self::BarCanyonKillBoss => "mails_barcanyonkillboss",
        }
    }
}

impl fmt::Display for MailType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Battle => "Battle",
            Self::DuelBattle2 => "DuelBattle2",
            Self::BarCanyonKillBoss => "BarCanyonKillBoss",
        };
        write!(f, "{label}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_type_from_str_parses_known_values() {
        assert_eq!(MailType::from_str("Battle"), Some(MailType::Battle));
        assert_eq!(
            MailType::from_str("DuelBattle2"),
            Some(MailType::DuelBattle2)
        );
        assert_eq!(
            MailType::from_str("BarCanyonKillBoss"),
            Some(MailType::BarCanyonKillBoss)
        );
        assert_eq!(MailType::from_str("Unknown"), None);
    }

    #[test]
    fn collection_name_matches_expected() {
        assert_eq!(MailType::Battle.collection_name(), "mails_battle");
        assert_eq!(MailType::DuelBattle2.collection_name(), "mails_duelbattle2");
        assert_eq!(
            MailType::BarCanyonKillBoss.collection_name(),
            "mails_barcanyonkillboss"
        );
    }
}
