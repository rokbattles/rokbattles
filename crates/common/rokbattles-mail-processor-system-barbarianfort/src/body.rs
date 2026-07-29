//! Body parser for SystemBarbarianFort mail.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Number, Value};

use crate::{
    content::{
        require_body, require_child_object, require_number_field, require_string_field,
        require_u64_field,
    },
    templates::BODY_TEMPLATES,
};

/// Pulls position and target details out of the SystemBarbarianFort body.
#[derive(Debug, Default)]
pub struct BodyExtractor;

impl BodyExtractor {
    /// Creates a body extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BodyExtractor {
    fn section(&self) -> &'static str {
        "body"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let body = require_body(input)?;
        let position = require_child_object(body, "position")?;
        let pos_x = require_number_field(position, "X")?;
        let pos_y = require_number_field(position, "Y")?;
        let target_name = require_string_field(body, "targetName")?;
        let sub_type = require_u64_field(body, "subType")?;
        let sub_param = require_u64_field(body, "subParam")?;
        let content_params = body
            .get("content")
            .and_then(Value::as_str)
            .and_then(|content| extract_content_params(content, sub_param, &target_name));

        let mut section = Section::new();
        section.insert("pos", build_position(pos_x, pos_y));
        section.insert("target_name", Value::String(target_name));
        section.insert("sub_type", Value::from(sub_type));
        section.insert("sub_param", Value::from(sub_param));
        if let Some(params) = content_params {
            section.insert("content", build_content(params.percentage, params.tier, params.level));
        }
        Ok(section)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ContentParams {
    percentage: Number,
    tier: u64,
    level: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum TemplateToken<'a> {
    Literal(&'a str),
    Placeholder(&'a str),
}

fn build_position(x: Value, y: Value) -> Value {
    let mut position = Map::new();
    position.insert("x".to_string(), x);
    position.insert("y".to_string(), y);
    Value::Object(position)
}

fn build_content(percentage: Number, tier: u64, level: u64) -> Value {
    let mut content = Map::new();
    content.insert("percentage".to_string(), Value::Number(percentage));
    content.insert("tier".to_string(), Value::from(tier));
    content.insert("level".to_string(), Value::from(level));
    Value::Object(content)
}

fn extract_content_params(
    content: &str,
    sub_param: u64,
    target_name: &str,
) -> Option<ContentParams> {
    BODY_TEMPLATES.iter().find_map(|template| {
        match_template(template.trim(), content.trim(), sub_param, target_name)
    })
}

fn match_template(
    template: &str,
    content: &str,
    sub_param: u64,
    target_name: &str,
) -> Option<ContentParams> {
    let tokens = tokenize_template(template);
    let mut remaining = content;
    let mut percentage = None;
    let mut tier = None;
    let mut level = None;

    for (index, token) in tokens.iter().enumerate() {
        match token {
            TemplateToken::Literal(literal) => {
                remaining = remaining.strip_prefix(literal)?;
            }
            TemplateToken::Placeholder(name) => {
                let following_tokens = tokens.get(index + 1..).unwrap_or_default();
                let next_literal = following_tokens.iter().find_map(|token| match token {
                    TemplateToken::Literal(literal) if !literal.is_empty() => Some(*literal),
                    TemplateToken::Literal(_) | TemplateToken::Placeholder(_) => None,
                });
                let capture = match next_literal {
                    Some(literal) => {
                        let end = remaining.find(literal)?;
                        let (capture, rest) = remaining.split_at_checked(end)?;
                        remaining = rest;
                        capture
                    }
                    None => {
                        let capture = remaining;
                        remaining = "";
                        capture
                    }
                };

                match *name {
                    "p2" => level = parse_level(capture.trim()),
                    "p3" => percentage = parse_damage_percentage(capture.trim()),
                    "p4" => tier = capture.trim().parse::<u64>().ok(),
                    _ => {}
                }
            }
        }
    }

    if !remaining.trim().is_empty() {
        return None;
    }

    let level =
        level.or_else(|| parse_level(target_name)).or_else(|| (sub_param == 3).then_some(11))?;

    Some(ContentParams { percentage: percentage?, tier: tier?, level })
}

fn tokenize_template(template: &str) -> Vec<TemplateToken<'_>> {
    let mut tokens = Vec::new();
    let mut remaining = template;

    while let Some(start) = remaining.find('{') {
        let Some((literal, unmatched_placeholder)) = remaining.split_at_checked(start) else {
            return tokens;
        };
        if !literal.is_empty() {
            tokens.push(TemplateToken::Literal(literal));
        }

        let Some(after_start) = unmatched_placeholder.strip_prefix('{') else {
            return tokens;
        };
        let Some(end) = after_start.find('}') else {
            tokens.push(TemplateToken::Literal(unmatched_placeholder));
            return tokens;
        };
        let Some((placeholder, after_placeholder)) = after_start.split_at_checked(end) else {
            return tokens;
        };
        let Some(rest) = after_placeholder.strip_prefix('}') else {
            return tokens;
        };

        tokens.push(TemplateToken::Placeholder(placeholder));
        remaining = rest;
    }

    if !remaining.is_empty() {
        tokens.push(TemplateToken::Literal(remaining));
    }

    tokens
}

fn parse_damage_percentage(value: &str) -> Option<Number> {
    let mut trimmed = value.trim();
    if let Some(value) = trimmed.strip_prefix('%') {
        trimmed = value.trim_start();
    }
    if let Some(value) = trimmed.strip_suffix('%') {
        trimmed = value.trim_end();
    }
    if trimmed.contains('%') {
        return None;
    }
    let parsed = trimmed.parse::<f64>().ok()?;
    if !parsed.is_finite() {
        return None;
    }
    Number::from_f64(parsed)
}

fn parse_level(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if let Ok(level) = trimmed.parse::<u64>() {
        return Some(level);
    }

    let start = trimmed.find(|character: char| character.is_ascii_digit())?;
    let digits = trimmed.get(start..)?;
    let end = digits.find(|character: char| !character.is_ascii_digit()).unwrap_or(digits.len());
    digits.get(..end)?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rokbattles_mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn body_extractor_reads_fields() {
        let input = json!({
            "body": {
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "subParam": 1,
                "subType": 11,
                "targetName": "Level9"
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["pos"], json!({ "x": 1.25, "y": 2.75 }));
        assert_eq!(fields["target_name"], json!("Level9"));
        assert_eq!(fields["sub_type"], json!(11));
        assert_eq!(fields["sub_param"], json!(1));
    }

    #[test]
    fn body_extractor_reads_content_params() {
        let input = json!({
            "body": {
                "content": "Congratulations! The Level 7 barbarian fort at X:582 Y:629 has been destroyed by your mighty onslaught.\n\nYou dealt 52% of the total damage and as a result have received the following <color=#00980e>Tier 6</color> trophies:",
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "subParam": 1,
                "subType": 11,
                "targetName": "Level7"
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["content"], json!({ "percentage": 52.0, "tier": 6, "level": 7 }));
    }

    #[test]
    fn body_extractor_reads_marauder_content_params() {
        let input = json!({
            "body": {
                "content": "Congratulations! The Marauder Encampment at X:1172 Y:208 has been destroyed by your mighty onslaught. You dealt 15% of the total damage and as a result have received the following <color=#00980e>Tier 3</color> trophies:",
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "subParam": 3,
                "subType": 11,
                "targetName": "Level11"
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["content"], json!({ "percentage": 15.0, "tier": 3, "level": 11 }));
    }

    #[test]
    fn body_extractor_reads_motte_content_params() {
        let input = json!({
            "body": {
                "content": "Congratulations! The level 11 Motte at X:1172 Y:208 has been destroyed by your mighty onslaught. You dealt 15% of the total damage and as a result received <color=#00980e>level 3</color> plunder:",
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "subParam": 4,
                "subType": 11,
                "targetName": "Level11"
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["content"], json!({ "percentage": 15.0, "tier": 3, "level": 11 }));
    }

    #[test]
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                }
            }
        });
        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn body_extractor_rejects_missing_sub_type() {
        let input = json!({
            "body": {
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "subParam": 1,
                "targetName": "Level9"
            }
        });

        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "subType" }));
    }

    #[test]
    fn roundtrip_body_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/System/Persistent.Mail.87938122177133895831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["pos"], json!({ "x": 3867.797119140625, "y": 4096.7294921875 }));
        assert_eq!(fields["target_name"], json!("Level9"));
        assert_eq!(fields["sub_type"], json!(11));
        assert_eq!(fields["sub_param"], json!(1));
        assert_eq!(fields["content"], json!({ "percentage": 52.0, "tier": 6, "level": 9 }));
    }

    #[test]
    fn localized_template_matcher_extracts_content_params() {
        let content = "执政官！位于X:582 Y:629的等级7野蛮人城寨在您的猛攻之下已经被摧毁，您在这次战役中总共造成了52%的伤害，因此获得了<color=#00980e>6阶</color>的战利品：";

        let params = extract_content_params(content, 1, "Level7").expect("params");

        assert_eq!(
            params,
            ContentParams {
                percentage: Number::from_f64(52.0).expect("valid percentage"),
                tier: 6,
                level: 7
            }
        );
    }

    #[test]
    fn localized_template_matcher_extracts_turkish_prefix_percentage() {
        let content = "Tebrikler! X:580 Y:552 konumundaki 7. Seviye barbar kalesi şiddetli saldırın sayesinde yok edildi.\n\nToplam hasarın %16 kısmını verdiğin için aşağıdaki <color=#00980e>Katman 3</color> ödüllerini aldın:";

        let params = extract_content_params(content, 1, "Seviye7").expect("params");

        assert_eq!(
            params,
            ContentParams {
                percentage: Number::from_f64(16.0).expect("valid percentage"),
                tier: 3,
                level: 7
            }
        );
    }

    #[test]
    fn localized_template_matcher_defaults_marauder_level() {
        let content = "Congratulations! The Marauder Encampment at X:1172 Y:208 has been destroyed by your mighty onslaught. You dealt 15% of the total damage and as a result have received the following <color=#00980e>Tier 3</color> trophies:";

        let params = extract_content_params(content, 3, "Marauder Encampment").expect("params");

        assert_eq!(
            params,
            ContentParams {
                percentage: Number::from_f64(15.0).expect("valid percentage"),
                tier: 3,
                level: 11
            }
        );
    }
}
