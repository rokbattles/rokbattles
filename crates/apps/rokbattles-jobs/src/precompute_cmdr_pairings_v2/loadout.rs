use std::collections::HashMap;

use mongodb::bson::Bson;
use rokbattles_bson::bson_to_i64;
use serde::Deserialize;

use super::{
    catalog::Catalogs,
    model::{InscriptionRarity, LoadoutBucket, MonthLoadouts, canonical_formation_id},
};

const LEGENDARY_EQUIPMENT_QUALITY: i64 = 5;

#[derive(Debug, Deserialize)]
pub(super) struct ProjectedLoadout {
    pub(super) d: i64,
    pub(super) c: i64,
    pub(super) u: i64,
    #[serde(default)]
    pub(super) f: i64,
    pub(super) e: Option<String>,
    #[serde(default)]
    pub(super) a: Vec<Armament>,
}

pub(super) fn map_projected_loadout(value: &Bson) -> Result<ProjectedLoadout, String> {
    let values = value.as_array().ok_or("loadout record is not an array")?;
    if values.len() != 6 {
        return Err(format!("loadout record has {} fields instead of 6", values.len()));
    }
    let integer = |index: usize, name: &str| {
        bson_to_i64(&values[index]).ok_or_else(|| format!("loadout {name} is not an integer"))
    };
    let e = match &values[4] {
        Bson::String(value) => Some(value.clone()),
        Bson::Null => None,
        _ => return Err("loadout equipment is not a string or null".to_owned()),
    };
    let a = mongodb::bson::from_bson(values[5].clone())
        .map_err(|error| format!("invalid loadout armaments: {error}"))?;

    Ok(ProjectedLoadout {
        d: integer(0, "day")?,
        c: integer(1, "scenario")?,
        u: integer(2, "governor")?,
        f: integer(3, "formation")?,
        e,
        a,
    })
}

#[derive(Debug, Deserialize)]
pub(super) struct Armament {
    id: i64,
    affix: Option<String>,
    buffs: Option<String>,
}

#[derive(Debug)]
struct EquipmentToken {
    slot: i64,
    item_id: i64,
    attribute: i64,
}

pub(super) fn accumulate_snapshot(
    month: &mut MonthLoadouts,
    snapshot: &ProjectedLoadout,
    catalogs: &Catalogs,
) {
    let bucket = month.buckets.entry((snapshot.d, snapshot.c)).or_default();
    accumulate_formation(bucket, snapshot.f);
    accumulate_armaments(bucket, &snapshot.a, catalogs);
    accumulate_equipment(bucket, snapshot.e.as_deref(), catalogs);
}

fn accumulate_formation(bucket: &mut LoadoutBucket, formation_id: i64) {
    let formation_id = canonical_formation_id(formation_id);
    if formation_id > 0 {
        bucket.formation.sample += 1;
        *bucket.formation.counts.entry(formation_id).or_default() += 1;
    }
}

fn accumulate_armaments(bucket: &mut LoadoutBucket, armaments: &[Armament], catalogs: &Catalogs) {
    for armament in armaments.iter().filter(|armament| (1..=4).contains(&armament.id)) {
        let slot = bucket.armaments.entry(armament.id).or_default();
        slot.sample += 1;

        let mut rarities = parse_signed_integers(armament.affix.as_deref())
            .into_iter()
            .filter_map(|id| catalogs.inscriptions.get(&id).copied())
            .collect::<Vec<_>>();
        rarities.sort_by_key(|rarity| match rarity {
            InscriptionRarity::Common => 0,
            InscriptionRarity::Rare => 1,
            InscriptionRarity::Special => 2,
        });
        match rarities.as_slice() {
            [InscriptionRarity::Special] => slot.inscriptions.special += 1,
            [InscriptionRarity::Rare] => slot.inscriptions.rare += 1,
            [InscriptionRarity::Common] => slot.inscriptions.common += 1,
            [InscriptionRarity::Common, InscriptionRarity::Special] => {
                slot.inscriptions.special_common += 1;
            }
            [InscriptionRarity::Common, InscriptionRarity::Rare] => {
                slot.inscriptions.rare_common += 1;
            }
            [InscriptionRarity::Common, InscriptionRarity::Common] => {
                slot.inscriptions.common_common += 1;
            }
            _ => {}
        }

        let buffs =
            parse_buff_pairs(armament.buffs.as_deref()).into_iter().collect::<HashMap<_, _>>();
        for (id, value) in buffs {
            let Some(maximum) = catalogs.armament_max_rolls.get(&id) else {
                continue;
            };
            let accumulator = slot.buffs.entry(id).or_default();
            accumulator.observations += 1;
            accumulator.total_roll += value;
            accumulator.max_rolls += i64::from(value + 1e-9 >= *maximum);
        }
    }
}

fn accumulate_equipment(bucket: &mut LoadoutBucket, value: Option<&str>, catalogs: &Catalogs) {
    let tokens = parse_equipment(value)
        .into_iter()
        .filter(|token| (1..=8).contains(&token.slot))
        .collect::<Vec<_>>();
    let mut accessories = tokens
        .iter()
        .filter(|token| matches!(token.slot, 7 | 8))
        .map(|token| token.item_id)
        .collect::<Vec<_>>();
    if accessories.len() == 2 {
        accessories.sort_unstable();
        bucket.accessory_sample += 1;
        *bucket.accessory_pairs.entry((accessories[0], accessories[1])).or_default() += 1;
    }

    for token in tokens {
        let Some(quality) = catalogs.equipment_qualities.get(&token.item_id) else {
            continue;
        };
        let slot_id = if token.slot == 8 { 7 } else { token.slot };
        let slot = bucket.equipment.entry(slot_id).or_default();
        let special_talent_code = token.attribute.div_euclid(10);
        let iconic =
            if special_talent_code > 0 { token.attribute.rem_euclid(10) } else { token.attribute };

        *slot.items.entry(token.item_id).or_default() += 1;
        if *quality == LEGENDARY_EQUIPMENT_QUALITY {
            slot.count += 1;
            *slot.iconic.entry(iconic.max(0)).or_default() += 1;
        } else {
            slot.excluded += 1;
        }
        if special_talent_code > 0 {
            slot.special_talent += 1;
        } else {
            slot.normal += 1;
        }
    }
}

pub(super) fn pack_month(month: MonthLoadouts) -> Vec<Bson> {
    let mut records = Vec::new();
    for ((day, scenario), bucket) in month.buckets {
        if bucket.formation.sample > 0 {
            let mut record =
                vec![0_i64.into(), day.into(), scenario.into(), bucket.formation.sample.into()];
            for (id, count) in bucket.formation.counts {
                record.extend([id.into(), count.into()]);
            }
            records.push(Bson::Array(record));
        }

        for (slot_id, slot) in bucket.armaments {
            let mut buffs = Vec::with_capacity(slot.buffs.len() * 4);
            for (id, buff) in slot.buffs {
                buffs.extend([
                    id.into(),
                    buff.observations.into(),
                    buff.total_roll.into(),
                    buff.max_rolls.into(),
                ]);
            }
            records.push(Bson::Array(vec![
                1_i64.into(),
                day.into(),
                scenario.into(),
                slot_id.into(),
                slot.sample.into(),
                slot.inscriptions.special.into(),
                slot.inscriptions.rare.into(),
                slot.inscriptions.common.into(),
                slot.inscriptions.special_common.into(),
                slot.inscriptions.rare_common.into(),
                slot.inscriptions.common_common.into(),
                Bson::Array(buffs),
            ]));
        }

        for (slot_id, slot) in bucket.equipment {
            let mut items = Vec::with_capacity(slot.items.len() * 2);
            for (id, count) in slot.items {
                items.extend([id.into(), count.into()]);
            }
            let mut iconic = Vec::with_capacity(slot.iconic.len() * 2);
            for (level, count) in slot.iconic {
                iconic.extend([level.into(), count.into()]);
            }
            records.push(Bson::Array(vec![
                2_i64.into(),
                day.into(),
                scenario.into(),
                slot_id.into(),
                slot.count.into(),
                slot.excluded.into(),
                slot.special_talent.into(),
                slot.normal.into(),
                Bson::Array(items),
                Bson::Array(iconic),
            ]));
        }

        if bucket.accessory_sample > 0 {
            let mut pairs = Vec::with_capacity(bucket.accessory_pairs.len() * 3);
            for ((id, id2), count) in bucket.accessory_pairs {
                pairs.extend([id.into(), id2.into(), count.into()]);
            }
            records.push(Bson::Array(vec![
                3_i64.into(),
                day.into(),
                scenario.into(),
                bucket.accessory_sample.into(),
                Bson::Array(pairs),
            ]));
        }
    }
    records
}

fn parse_equipment(value: Option<&str>) -> Vec<EquipmentToken> {
    let Some(value) = value else {
        return Vec::new();
    };
    let value = value.trim().trim_start_matches('{').trim_end_matches('}');
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .filter_map(|part| {
            let mut fields = part.split(':').map(str::trim);
            let slot = fields.next()?.parse::<i64>().ok()?;
            let item_id = fields.next()?.split('_').next()?.parse::<i64>().ok()?;
            let attribute = fields.next()?.parse::<i64>().ok()?;
            Some(EquipmentToken { slot, item_id, attribute })
        })
        .collect()
}

fn parse_signed_integers(value: Option<&str>) -> Vec<i64> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut values = Vec::new();
    let mut current = String::new();
    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() || (character == '-' && current.is_empty()) {
            current.push(character);
        } else if !current.is_empty() {
            if let Ok(value) = current.parse::<i64>()
                && value > 0
            {
                values.push(value);
            }
            current.clear();
        }
    }
    values
}

fn parse_buff_pairs(value: Option<&str>) -> Vec<(i64, f64)> {
    let Some(value) = value else {
        return Vec::new();
    };
    value
        .split([';', ','])
        .filter_map(|part| {
            let mut fields = part.trim().split(['_', ':']);
            let id = fields.next()?.parse::<i64>().ok()?;
            let value = fields.next()?.parse::<f64>().ok()?;
            (id > 0 && value.is_finite() && value >= 0.0).then_some((id, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::precompute_cmdr_pairings_v2::model::PairingKey;

    #[test]
    fn accessory_pairs_are_order_agnostic_and_accessory_slots_are_combined() {
        let catalogs = Catalogs {
            inscriptions: HashMap::new(),
            armament_max_rolls: HashMap::new(),
            equipment_qualities: HashMap::from([(101, 5), (202, 5)]),
        };
        let mut bucket = LoadoutBucket::default();
        accumulate_equipment(&mut bucket, Some("{7:202_0:0,8:101_0:0}"), &catalogs);

        assert_eq!(bucket.accessory_pairs.get(&(101, 202)), Some(&1));
        assert_eq!(bucket.equipment.get(&7).map(|slot| slot.count), Some(2));
    }

    #[test]
    fn packed_equipment_uses_ids_and_counts_without_names() {
        let mut month = MonthLoadouts {
            pairing: PairingKey { primary: 1, secondary: 2 },
            month: 0,
            ..MonthLoadouts::default()
        };
        let bucket = month.buckets.entry((10, 1)).or_default();
        bucket.equipment.entry(1).or_default().items.insert(101, 3);

        let packed = pack_month(month);

        assert_eq!(packed.len(), 1);
        assert!(!format!("{packed:?}").contains("name"));
    }
}
