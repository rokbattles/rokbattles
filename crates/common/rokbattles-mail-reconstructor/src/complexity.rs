//! Preflight bounds for attacker-controlled JSON/protobuf expansion, shared by a mail.

use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::{
    ReconstructionError,
    artifact::DescriptorPool,
    protobuf::{FieldValue, fields},
};

const MAX_DEPTH: usize = 64;
const MAX_VALUES: usize = 500_000;
const MAX_VALUE_BYTES: usize = 64 * 1024 * 1024;

pub(crate) struct Budget {
    values: usize,
    bytes: usize,
    exhausted: bool,
}

impl Budget {
    pub(crate) fn new() -> Self {
        Self { values: MAX_VALUES, bytes: MAX_VALUE_BYTES, exhausted: false }
    }

    pub(crate) fn nodes(&mut self, count: usize, depth: usize) -> Result<(), ReconstructionError> {
        if depth > MAX_DEPTH || self.values < count {
            self.exhausted = true;
            return Err(ReconstructionError::BudgetExceeded);
        }
        self.values -= count;
        Ok(())
    }

    pub(crate) fn bytes(&mut self, count: usize) -> Result<(), ReconstructionError> {
        let Some(remaining) = self.bytes.checked_sub(count) else {
            self.exhausted = true;
            return Err(ReconstructionError::BudgetExceeded);
        };
        self.bytes = remaining;
        Ok(())
    }
}

pub(crate) fn json(data: &[u8], budget: &mut Budget) -> Result<(), ReconstructionError> {
    let mut decoder = serde_json::Deserializer::from_slice(data);
    let result = Seed { budget, depth: 0 }.deserialize(&mut decoder).and_then(|()| decoder.end());
    match result {
        Ok(()) => Ok(()),
        Err(_) if budget.exhausted => Err(ReconstructionError::BudgetExceeded),
        Err(error) => Err(ReconstructionError::InvalidBodyJson(error)),
    }
}

pub(crate) fn info(text: &str, budget: &mut Budget) -> Result<(), ReconstructionError> {
    match json(text.as_bytes(), budget) {
        Err(ReconstructionError::BudgetExceeded) => Err(ReconstructionError::BudgetExceeded),
        _ => budget.bytes(text.len()),
    }
}

struct Seed<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.budget
            .nodes(1, self.depth)
            .map_err(|_error| de::Error::custom("value budget exhausted"))?;
        decoder.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bounded JSON value")
    }
    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _value: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _value: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, _value: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<(), E> {
        self.budget.bytes(value.len()).map_err(|_error| de::Error::custom("value budget exhausted"))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        while sequence
            .next_element_seed(Seed { budget: &mut *self.budget, depth: self.depth + 1 })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        // Keys are strings in JSON. IgnoredAny validates them without building a
        // retained object. Their encoded bytes remain covered by the input cap.
        while map.next_key::<IgnoredAny>()?.is_some() {
            map.next_value_seed(Seed { budget: &mut *self.budget, depth: self.depth + 1 })?;
        }
        Ok(())
    }
}

pub(crate) fn message(
    data: &[u8],
    name: &str,
    descriptors: &DescriptorPool,
    budget: &mut Budget,
) -> Result<(), ReconstructionError> {
    message_at(data, name, descriptors, budget, 0)
}

fn message_at(
    data: &[u8],
    name: &str,
    descriptors: &DescriptorPool,
    budget: &mut Budget,
    depth: usize,
) -> Result<(), ReconstructionError> {
    let descriptor = descriptors.message(name)?;
    budget.nodes(1 + descriptor.fields.len(), depth)?;
    for field in &descriptor.fields {
        budget.bytes(field.name.len())?;
    }

    for encoded in fields(data) {
        let encoded = encoded?;
        budget.nodes(1, depth)?;
        let Some(field) = descriptor.fields.iter().find(|field| field.number == encoded.number)
        else {
            continue;
        };
        match (field.field_type, encoded.value) {
            (11, FieldValue::Bytes(data)) => {
                message_at(data, &field.type_name, descriptors, budget, depth + 1)?
            }
            (9, FieldValue::Bytes(text)) => {
                budget.nodes(1, depth)?;
                budget.bytes(text.len())?;
                if field.name == "Kvs" && !text.is_empty() {
                    json(text, budget)?;
                }
            }
            (12, FieldValue::Bytes(bytes)) => {
                budget.nodes(1, depth)?;
                let encoded = bytes
                    .len()
                    .checked_add(2)
                    .and_then(|value| value.checked_div(3))
                    .and_then(|value| value.checked_mul(4))
                    .ok_or(ReconstructionError::BudgetExceeded)?;
                budget.bytes(encoded)?;
            }
            (_, FieldValue::Bytes(packed)) if field.label == 3 => {
                let width = match field.field_type {
                    1 | 6 | 16 => 8,
                    2 | 7 | 15 => 4,
                    _ => 1,
                };
                // One-byte varints are the highest expansion case; conservative
                // counting avoids allocating a decoded packed vector first.
                budget.nodes(packed.len().div_ceil(width), depth)?;
            }
            _ => budget.nodes(1, depth)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_json_and_wide_arrays_fail_before_value_allocation() {
        let nested = format!("{}0{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
        assert!(matches!(
            json(nested.as_bytes(), &mut Budget::new()),
            Err(ReconstructionError::BudgetExceeded)
        ));
        let wide = format!("[{}0]", "0,".repeat(MAX_VALUES));
        assert!(matches!(
            json(wide.as_bytes(), &mut Budget::new()),
            Err(ReconstructionError::BudgetExceeded)
        ));
    }

    #[test]
    fn budget_is_shared_across_independent_json_sections() {
        let mut budget = Budget { values: 4, bytes: 100, exhausted: false };
        json(b"[1,2]", &mut budget).expect("first section");
        assert!(matches!(json(b"[3]", &mut budget), Err(ReconstructionError::BudgetExceeded)));
    }

    #[test]
    fn largest_checked_in_battle_shape_fits_the_value_budget() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.8421200117137313126.json");
        let bytes = std::fs::read(path).expect("battle fixture");
        json(&bytes, &mut Budget::new()).expect("legitimate large battle");
    }

    #[test]
    fn cyclic_descriptor_data_and_packed_expansion_are_bounded() {
        use crate::artifact::{DynamicField, DynamicMessage};
        let descriptors = DescriptorPool::test_pool(
            "Node",
            DynamicMessage {
                fields: vec![
                    DynamicField {
                        name: "Children".into(),
                        number: 1,
                        label: 3,
                        field_type: 11,
                        type_name: "Node".into(),
                    },
                    DynamicField {
                        name: "Numbers".into(),
                        number: 2,
                        label: 3,
                        field_type: 5,
                        type_name: String::new(),
                    },
                ],
            },
        );
        fn field(number: u8, payload: &[u8]) -> Vec<u8> {
            let mut output = vec![number << 3 | 2];
            let mut length = payload.len();
            while length >= 128 {
                output.push((length as u8 & 0x7f) | 0x80);
                length >>= 7;
            }
            output.push(length as u8);
            output.extend(payload);
            output
        }
        let mut nested = Vec::new();
        for _ in 0..MAX_DEPTH + 1 {
            nested = field(1, &nested);
        }
        assert!(matches!(
            message(&nested, "Node", &descriptors, &mut Budget::new()),
            Err(ReconstructionError::BudgetExceeded)
        ));
        let packed = field(2, &vec![0; MAX_VALUES + 1]);
        assert!(matches!(
            message(&packed, "Node", &descriptors, &mut Budget::new()),
            Err(ReconstructionError::BudgetExceeded)
        ));
        let unknown = [24, 0].repeat(MAX_VALUES + 1);
        assert!(matches!(
            message(&unknown, "Node", &descriptors, &mut Budget::new()),
            Err(ReconstructionError::BudgetExceeded)
        ));
    }
}
