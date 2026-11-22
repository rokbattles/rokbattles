//! MongoDB projection expressions shared by Combat Lab jobs.

use mongodb::bson::{Bson, Document, doc};

pub(crate) fn commander_condition(primary: &str, secondary: &str, ids: &[Bson]) -> Document {
    doc! {
        "$and": [
            { "$in": [primary, ids.to_vec()] },
            { "$in": [secondary, ids.to_vec()] },
            { "$ne": [primary, secondary] },
        ]
    }
}

pub(crate) fn conditional_entry(condition: Document, entry: Document) -> Document {
    doc! {
        "$cond": [
            condition,
            Bson::Array(vec![Bson::Document(entry)]),
            Bson::Array(Vec::new()),
        ]
    }
}

pub(crate) fn normalize_timestamp_expr(value: Document) -> Document {
    doc! {
        "$let": {
            "vars": {
                "raw": value.clone(),
                "abs": { "$abs": value },
            },
            "in": {
                "$switch": {
                    "branches": [
                        {
                            "case": { "$lt": ["$$abs", 1_000_000_000_000_f64] },
                            "then": { "$multiply": ["$$raw", 1000.0] },
                        },
                        {
                            "case": { "$gte": ["$$abs", 100_000_000_000_000_000_f64] },
                            "then": { "$divide": ["$$raw", 1_000_000.0] },
                        },
                        {
                            "case": { "$gte": ["$$abs", 100_000_000_000_000_f64] },
                            "then": { "$divide": ["$$raw", 1000.0] },
                        },
                    ],
                    "default": "$$raw",
                }
            }
        }
    }
}
