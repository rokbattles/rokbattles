//! Data types used by processor outputs.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// A collection of extracted fields for a processor section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The underlying section data, either object fields or an array payload.
    data: SectionData,
}

#[derive(Debug, Clone, PartialEq)]
enum SectionData {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
}

impl Section {
    /// Create an empty section.
    pub fn new() -> Self {
        Self {
            data: SectionData::Object(BTreeMap::new()),
        }
    }

    /// Create a section backed by an array payload.
    pub fn from_array(values: Vec<Value>) -> Self {
        Self {
            data: SectionData::Array(values),
        }
    }

    /// Insert a value into the section object.
    ///
    /// # Panics
    /// Panics if the section is backed by an array.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        match &mut self.data {
            SectionData::Object(fields) => fields.insert(key.into(), value),
            SectionData::Array(_) => panic!("attempted to insert into an array section"),
        }
    }

    /// Read the extracted fields for an object-backed section.
    ///
    /// # Panics
    /// Panics if the section is backed by an array.
    pub fn fields(&self) -> &BTreeMap<String, Value> {
        match &self.data {
            SectionData::Object(fields) => fields,
            SectionData::Array(_) => panic!("attempted to read fields from an array section"),
        }
    }

    /// Read the array payload for an array-backed section.
    pub fn array(&self) -> Option<&[Value]> {
        match &self.data {
            SectionData::Array(values) => Some(values.as_slice()),
            SectionData::Object(_) => None,
        }
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for Section {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.data {
            SectionData::Object(fields) => fields.serialize(serializer),
            SectionData::Array(values) => values.serialize(serializer),
        }
    }
}

/// The full processed output containing all sections.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(transparent)]
pub struct ProcessedMail {
    /// Sections keyed by their extractor name.
    sections: BTreeMap<String, Section>,
}

impl ProcessedMail {
    /// Create an empty processed mail object.
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    /// Insert a new section.
    pub fn insert(&mut self, key: impl Into<String>, section: Section) -> Option<Section> {
        self.sections.insert(key.into(), section)
    }

    /// Read the processed sections.
    pub fn sections(&self) -> &BTreeMap<String, Section> {
        &self.sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn section_serializes_as_field_map() {
        let mut section = Section::new();
        section.insert("mail_id", json!("mail-2"));
        let encoded = serde_json::to_value(section).expect("serialize section");
        assert_eq!(encoded, json!({ "mail_id": "mail-2" }));
    }

    #[test]
    fn section_serializes_as_array() {
        let section = Section::from_array(vec![json!({ "id": 1 }), json!({ "id": 2 })]);
        let encoded = serde_json::to_value(section).expect("serialize section");
        assert_eq!(encoded, json!([{ "id": 1 }, { "id": 2 }]));
    }

    #[test]
    fn processed_mail_serializes_as_section_map() {
        let mut section = Section::new();
        section.insert("mail_id", json!("mail-3"));
        let mut processed = ProcessedMail::new();
        processed.insert("metadata", section);
        let encoded = serde_json::to_value(processed).expect("serialize processed");
        assert_eq!(encoded, json!({ "metadata": { "mail_id": "mail-3" } }));
    }

    #[test]
    fn processed_mail_serializes_array_section() {
        let section = Section::from_array(vec![json!({ "player_id": 1 })]);
        let mut processed = ProcessedMail::new();
        processed.insert("opponents", section);
        let encoded = serde_json::to_value(processed).expect("serialize processed");
        assert_eq!(encoded, json!({ "opponents": [{ "player_id": 1 }] }));
    }
}
