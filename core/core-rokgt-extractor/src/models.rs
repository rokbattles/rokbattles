use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Role {
    pub(crate) app_id: u64,
    pub(crate) app_uid: String,
    pub(crate) uid: u64,
    pub(crate) svr_id: u32,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) power: Option<u64>,
}

/// Member data with the queried kingdom attached.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KingdomMember {
    /// Kingdom used for the query.
    pub kingdom: u32,
    /// Raw fields returned by `kindomMember`.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// One fetched block of member records.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KingdomMemberBatch {
    /// Kingdoms requested in this batch.
    pub server_ids: Vec<u32>,
    /// Member records returned for the batch.
    pub members: Vec<KingdomMember>,
}

/// Date range sent to `kindomMember`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemberDateRange {
    pub(crate) start: String,
    pub(crate) end: String,
}

impl MemberDateRange {
    pub(crate) fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self { start: start.into(), end: end.into() }
    }
}

/// Group member records by kingdom.
pub fn group_members_by_kingdom(members: Vec<KingdomMember>) -> BTreeMap<u32, Vec<KingdomMember>> {
    let mut grouped = BTreeMap::<u32, Vec<KingdomMember>>::new();
    for member in members {
        grouped.entry(member.kingdom).or_default().push(member);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_members_by_kingdom() {
        let members = vec![
            KingdomMember { kingdom: 2, fields: Map::new() },
            KingdomMember { kingdom: 1, fields: Map::new() },
            KingdomMember { kingdom: 2, fields: Map::new() },
        ];

        let grouped = group_members_by_kingdom(members);

        assert_eq!(grouped.get(&2).map(Vec::len), Some(2));
    }
}
