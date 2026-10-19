//! Extracts Kau Karuak trial level and difficulty from the subtitle.

use rokbattles_mail_sdk::{
    ExtractError, Extractor, Section, require_child_object, require_object,
    require_u64_or_string_field,
};
use serde_json::{Value, json};

use crate::templates::{DIFFICULTIES, LOCALES};

#[derive(Debug, Default)]
pub(crate) struct BodyExtractor;

impl Extractor for BodyExtractor {
    fn section(&self) -> &'static str {
        "body"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let body = require_child_object(require_object(input)?, "body")?;

        let mut section = Section::new();
        section.insert("sub_type", require_u64_or_string_field(body, "subType")?.into());
        section.insert("sub_param", require_u64_or_string_field(body, "subParam")?.into());

        // A translation mismatch must not prevent collection of the reward data.
        if let Some(content) = body.get("subTitle").and_then(Value::as_str).and_then(parse_subtitle)
        {
            section.insert("content", content);
        }

        Ok(section)
    }
}

fn parse_subtitle(subtitle: &str) -> Option<Value> {
    let subtitle = subtitle.replace('\u{a0}', " ");
    let subtitle = subtitle.trim();

    for locale in LOCALES {
        for (localized, difficulty) in locale.difficulties.iter().zip(DIFFICULTIES) {
            let template = locale.subtitle.replace("{p1}", localized).replace('\u{a0}', " ");
            let (prefix, suffix) = template.split_once("{p2}")?;

            let Some(level) = subtitle.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix))
            else {
                continue;
            };

            let level = level.trim();
            if !level.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }

            if let Ok(level) = level.parse::<u64>()
                && level > 0
            {
                return Some(json!({ "level": level, "difficulty": difficulty }));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_subtitles_with_different_placeholder_orders() {
        for (subtitle, level, difficulty) in [
            ("Lvl 30 Trial Completed At HELL Difficulty", 30, "hell"),
            ("Lv.-17-Prüfung auf Schwierigkeitsgrad SCHWER abgeschlossen", 17, "hard"),
            ("Épreuve de niv. 15 complétée à la difficulté EFFROYABLE", 15, "nightmare"),
            ("您已经完成了普通难度等级15的试炼", 15, "normal"),
            ("ヘル難易度レベル15の試練をクリアしました", 15, "hell"),
            ("이미 매우 어려움 난이도 15레벨의 시련을 완료했습니다.", 15, "nightmare"),
        ] {
            assert_eq!(
                parse_subtitle(subtitle),
                Some(json!({"level": level, "difficulty": difficulty}))
            );
        }
    }

    #[test]
    fn parses_all_official_locales_and_difficulties() {
        assert_eq!(LOCALES.len(), 18);

        for locale in LOCALES {
            for (localized, difficulty) in locale.difficulties.iter().zip(DIFFICULTIES) {
                for level in [1, 15, 16, 30] {
                    let subtitle = locale
                        .subtitle
                        .replace("{p1}", localized)
                        .replace("{p2}", &level.to_string());

                    assert_eq!(
                        parse_subtitle(&subtitle),
                        Some(json!({"level": level, "difficulty": difficulty})),
                        "{subtitle}"
                    );
                }
            }
        }
    }

    #[test]
    fn normalizes_outer_whitespace_and_non_breaking_spaces() {
        assert_eq!(
            parse_subtitle(" \nLvl\u{a0}15 Trial Completed At NORMAL Difficulty\n"),
            Some(json!({"level": 15, "difficulty": "normal"}))
        );
    }

    #[test]
    fn rejects_unknown_difficulty_malformed_levels_and_partial_matches() {
        for subtitle in [
            "Lvl 15 Trial Completed At UNKNOWN Difficulty",
            "Lvl 0 Trial Completed At HARD Difficulty",
            "Lvl -1 Trial Completed At HARD Difficulty",
            "Lvl +1 Trial Completed At HARD Difficulty",
            "Lvl 1.5 Trial Completed At HARD Difficulty",
            "Lvl 18446744073709551616 Trial Completed At HARD Difficulty",
            "Lvl 15 Trial Completed At HARD Difficulty extra",
            "Challenge 15 (HARD) Complete",
            "",
        ] {
            assert_eq!(parse_subtitle(subtitle), None, "{subtitle}");
        }
    }
}
