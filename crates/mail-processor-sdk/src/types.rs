//! Output types shared by the processors.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

/// Data for one processed section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Backing data for the section, either fields or an array payload.
    data: SectionData,
}

#[derive(Debug, Clone, PartialEq)]
enum SectionData {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
}

impl Section {
    /// Creates an empty object-backed section.
    #[must_use]
    pub fn new() -> Self {
        Self { data: SectionData::Object(BTreeMap::new()) }
    }

    /// Creates a section backed by an array payload.
    #[must_use]
    pub fn from_array(values: Vec<Value>) -> Self {
        Self { data: SectionData::Array(values) }
    }

    /// Creates an object-backed section from existing fields.
    #[must_use]
    pub fn from_fields(fields: Map<String, Value>) -> Self {
        Self { data: SectionData::Object(fields.into_iter().collect()) }
    }

    /// Inserts a value into an object-backed section.
    ///
    /// # Panics
    /// Panics if the section is backed by an array.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        match &mut self.data {
            SectionData::Object(fields) => fields.insert(key.into(), value),
            SectionData::Array(_) => panic!("attempted to insert into an array section"),
        }
    }

    /// Returns the fields when the section is object-backed.
    #[must_use]
    pub fn try_fields(&self) -> Option<&BTreeMap<String, Value>> {
        match &self.data {
            SectionData::Object(fields) => Some(fields),
            SectionData::Array(_) => None,
        }
    }

    /// Returns the fields for an object-backed section.
    ///
    /// # Panics
    /// Panics if the section is backed by an array.
    #[must_use]
    pub fn fields(&self) -> &BTreeMap<String, Value> {
        self.try_fields().expect("attempted to read fields from an array section")
    }

    /// Returns the array payload for an array-backed section.
    #[must_use]
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

/// The full processed mail, keyed by section name.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(transparent)]
pub struct ProcessedMail {
    /// Sections keyed by extractor name.
    sections: BTreeMap<String, Section>,
}

impl ProcessedMail {
    /// Creates an empty processed mail object.
    #[must_use]
    pub fn new() -> Self {
        Self { sections: BTreeMap::new() }
    }

    /// Inserts a section.
    pub fn insert(&mut self, key: impl Into<String>, section: Section) -> Option<Section> {
        self.sections.insert(key.into(), section)
    }

    /// Returns the processed sections.
    #[must_use]
    pub fn sections(&self) -> &BTreeMap<String, Section> {
        &self.sections
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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

    #[test]
    fn section_try_fields_returns_none_for_array_sections() {
        let section = Section::from_array(vec![json!(1)]);
        assert!(section.try_fields().is_none());
    }
}
