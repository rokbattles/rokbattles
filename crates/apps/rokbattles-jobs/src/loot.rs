//! Reward identifiers, quantities, and rates used by the loot precomputations.

use mongodb::bson::{Bson, Document, doc};
use rokbattles_bson::{bson_to_f64, bson_to_i32_exact, bson_to_i64_exact};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LootKey {
    pub reward_type: i32,
    pub sub_type: i32,
}

#[derive(Debug, Default)]
pub(crate) struct LootStats {
    pub seen: usize,
    pub total_quantity: i64,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl LootStats {
    pub fn record(&mut self, quantity: i64) {
        self.seen += 1;
        self.total_quantity += quantity;
        self.min = Some(self.min.map_or(quantity, |v| v.min(quantity)));
        self.max = Some(self.max.map_or(quantity, |v| v.max(quantity)));
    }
}

pub(crate) fn direct_i32(document: &Document, key: &str) -> Option<i32> {
    document.get(key).and_then(bson_to_i32_exact)
}

pub(crate) fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64_exact)
}

pub(crate) fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

pub(crate) fn rate(part: usize, total: usize) -> f64 {
    if total == 0 { 0.0 } else { part as f64 / total as f64 }
}

fn rate_i64(total_quantity: i64, seen: usize) -> f64 {
    if seen == 0 { 0.0 } else { total_quantity as f64 / seen as f64 }
}

pub(crate) fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(crate) fn build_loot_document(
    key: LootKey,
    stats: &LootStats,
    results: usize,
    default_quantity: i64,
) -> Document {
    doc! {
        "type": key.reward_type,
        "sub_type": key.sub_type,
        "results": usize_to_i64(stats.seen),
        "drop_rate": rate(stats.seen, results),
        "quantity": {
            "min": stats.min.unwrap_or(default_quantity),
            "max": stats.max.unwrap_or(default_quantity),
        },
        "total_quantity": stats.total_quantity,
        "average_quantity": rate_i64(stats.total_quantity, stats.seen),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Reward {
    pub key: LootKey,
    pub quantity: i64,
}

impl Reward {
    fn parse(value: &Bson) -> Option<Self> {
        let doc = value.as_document()?;
        Some(Self {
            key: LootKey {
                reward_type: direct_i32(doc, "type")?,
                sub_type: direct_i32(doc, "sub_type")?,
            },
            quantity: direct_i64(doc, "value")?,
        })
    }

    pub fn positive(value: &Bson) -> Option<Self> {
        let reward = Self::parse(value)?;
        (reward.quantity > 0).then_some(reward)
    }

    pub fn with_empty_slot(value: &Bson) -> Option<Self> {
        let reward = Self::parse(value)?;
        let empty = reward.key.reward_type == 2 && reward.key.sub_type == 0 && reward.quantity == 0;
        (empty || (reward.quantity > 0 && reward.key.sub_type != 0)).then_some(reward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_reward_rules_distinguish_positive_subtype_zero_from_empty_slots() {
        let reward = |quantity| Bson::Document(doc! {"type": 2, "sub_type": 0, "value": quantity});
        assert!(Reward::positive(&reward(1)).is_some());
        assert!(Reward::with_empty_slot(&reward(1)).is_none());
        assert!(Reward::positive(&reward(0)).is_none());
        assert!(Reward::with_empty_slot(&reward(0)).is_some());
    }
}
