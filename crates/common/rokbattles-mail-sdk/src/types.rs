//! Owned sections and their serialized representation.
//!
//! BTreeMaps give section names and object fields a stable sorted order. Arrays
//! keep their input order. Serialization exposes these containers directly.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

/// An owned JSON object or array produced by an extractor.
///
/// [`Self::new`] and [`Self::from_fields`] create objects; [`Self::from_array`]
/// creates an array. Use [`Self::try_fields`] and [`Self::array`] when the shape
/// is unknown. Serialization writes the contained object or array directly,
/// without a variant tag or `data` wrapper.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The section shape is fixed at construction.
    data: SectionData,
}

#[derive(Debug, Clone, PartialEq)]
enum SectionData {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
}

impl Section {
    /// Creates an empty object section, as does [`Default::default`].
    #[must_use]
    pub fn new() -> Self {
        Self { data: SectionData::Object(BTreeMap::new()) }
    }

    /// Takes ownership of an array, preserving element order.
    #[must_use]
    pub fn from_array(values: Vec<Value>) -> Self {
        Self { data: SectionData::Array(values) }
    }

    /// Takes ownership of object fields and stores them in sorted key order.
    #[must_use]
    pub fn from_fields(fields: Map<String, Value>) -> Self {
        Self { data: SectionData::Object(fields.into_iter().collect()) }
    }

    /// Inserts a field into an object section.
    ///
    /// Replaces an existing value and returns it, or returns `None` for a new key.
    ///
    /// # Panics
    ///
    /// Panics if the section is backed by an array.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        match &mut self.data {
            SectionData::Object(fields) => fields.insert(key.into(), value),
            SectionData::Array(_) => panic!("attempted to insert into an array section"),
        }
    }

    /// Borrows the sorted fields of an object section, or returns `None` for an array.
    #[must_use]
    pub fn try_fields(&self) -> Option<&BTreeMap<String, Value>> {
        match &self.data {
            SectionData::Object(fields) => Some(fields),
            SectionData::Array(_) => None,
        }
    }

    /// Borrows the sorted fields of an object section.
    ///
    /// Use [`Self::try_fields`] when the section might contain an array.
    ///
    /// # Panics
    ///
    /// Panics if the section is backed by an array.
    #[must_use]
    pub fn fields(&self) -> &BTreeMap<String, Value> {
        self.try_fields().expect("attempted to read fields from an array section")
    }

    /// Borrows the array elements, or returns `None` for an object section.
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
        // Keep the internal enum out of the serialized schema.
        match &self.data {
            SectionData::Object(fields) => fields.serialize(serializer),
            SectionData::Array(values) => values.serialize(serializer),
        }
    }
}

/// Named sections serialized as a JSON object.
///
/// The section map is stored in sorted key order and serializes without a
/// `sections` wrapper. Each value uses its section's object or array shape.
///
/// # Examples
///
/// ```
/// use rokbattles_mail_sdk::{ProcessedMail, Section};
/// use serde_json::json;
///
/// let mut mail = ProcessedMail::new();
/// mail.insert("opponents", Section::from_array(vec![json!({ "id": 1 })]));
/// assert_eq!(serde_json::to_value(mail)?, json!({ "opponents": [{ "id": 1 }] }));
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(transparent)]
pub struct ProcessedMail {
    /// Sections keyed by their output names.
    sections: BTreeMap<String, Section>,
}

impl ProcessedMail {
    /// Creates an empty processed mail object.
    #[must_use]
    pub fn new() -> Self {
        Self { sections: BTreeMap::new() }
    }

    /// Inserts a section, returning the previous section if the name already exists.
    ///
    /// This method replaces duplicates; [`crate::Processor`] enforces unique
    /// extractor section names when assembling output.
    pub fn insert(&mut self, key: impl Into<String>, section: Section) -> Option<Section> {
        self.sections.insert(key.into(), section)
    }

    /// Borrows the sections in sorted name order.
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
