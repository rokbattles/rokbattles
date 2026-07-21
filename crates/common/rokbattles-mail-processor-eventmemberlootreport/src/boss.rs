//! GVE alliance boss identification from localized report subtitles.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section};
use serde_json::Value;

use crate::content::{require_content, require_string_field};

struct Boss {
    id: u64,
    localized_names: &'static [&'static str],
}

const BOSSES: &[Boss] = &[
    Boss {
        id: 30001,
        localized_names: &[
            "اندال قبضة السيف",
            "Klingenfaust Andaal",
            "Bladefist Andaal",
            "Andaal Puño Cortante",
            "Andaal Poing-de-sang",
            "Andaal Pugno di Lame",
            "シザーハンド・アンドリュー",
            "가위손 앤드류",
            "Andaal Siekająca Pięść",
            "Andaal Punhos de Lâminas",
            "Андаал Гром-Кулак",
            "หมัดคมดาบอันดาล",
            "Keskin Yumruk Andaal",
            "Tiễn Đao Thủ Andaal",
            "剪刀手 · 安德魯",
            "剪刀手 · 安德鲁",
            // Historical names
            "\"Fists of Blades\" Andaal",
            "\"قبضة السيوف\" آندال",
            "„Klingenfäuste\" Andaal",
            "\"Puños de Espadas\" Andaal",
            "Andaal « Poings tranchants »",
            "\"Pugni di Lame\" Andaal",
            "\"Tumbukan Bilah\" Andaal",
            "Andaal Siekające Pięści",
            "Andaal \"Punhos de lâminas\"",
            "Андаал «Кулаки Ярости»",
            "\"กำปั้นใบมีด\" อันดาล",
            "\"Bıçak Yumruğu\" Andaal",
            "\"Sức mạnh lưỡi đao\" Andaal",
        ],
    },
    Boss {
        id: 30002,
        localized_names: &[
            "لوكار حارس الدب",
            "Bärenwächter Lukor",
            "Bearkeeper Lukor",
            "Cuidador de Osos Lukor",
            "Dresseur d'ours Lukor",
            "Guardiano degli Orsi Lukor",
            "ベアキーパー・ルクルス",
            "베어워커 루크로스",
            "Penjaga Beruang Lukor",
            "Opiekun niedźwiedzi Lukor",
            "Lukor, domador de ursos",
            "Зверолов Лукор",
            "คนเลี้ยงหมี ลูคอร์",
            "Ayı Bakıcısı Lukor",
            "Chăn gấu Lukor",
            "馭熊者 · 魯克魯斯",
            "驭熊者 · 鲁克鲁斯",
            // Historical names
            "Смотритель медведей Лукор",
        ],
    },
    Boss {
        id: 30003,
        localized_names: &[
            "موردوس الدرع الغاشم",
            "Rohling Murdos",
            "Bruteshield Murdos",
            "Murdos Granescudo",
            "Murdos le Rempart",
            "Murdos il Duro",
            "残酷ガーディアン・モードー",
            "잔인한 호위군 모도",
            "Mocarz Murdos",
            "Murdos de Escudo Bruto",
            "Мурдос Несгибаемый",
            "บรูตชีลด์มูร์โดส",
            "Sarsılmaz Murdos",
            "Cấm Vệ Tàn Khốc Murdos",
            "殘酷禁衛 · 摩多",
            "残酷禁卫 · 摩多",
            // Historical names
            "Shield Chieftain Murdos",
            "لصوص موردوس المدرعين",
            "酋长禁卫 · 摩多",
            "Schildhäuptling Murdos",
            "Jefe de Escudos Murdos",
            "Porte-bouclier Murdos",
            "Capo Scudo Murdos",
            "추장 호위군 모도",
            "Ketua Perisai Murdos",
            "Wódz z tarczą Murdos",
            "Murdos, chefe de escudos",
            "Вождь-защитник Мурдос",
            "ประมุขเกราะ มูร์โดส",
            "Kalkan Lideri Murdos",
            "Thủ lĩnh khiên Murdos",
            "酋長禁衛 · 摩多",
        ],
    },
    Boss {
        id: 30004,
        localized_names: &[
            "باتشي مُعالج الحرب",
            "Kriegshüter Pache",
            "Warmender Pache",
            "Pache el Curandero",
            "Pache le Médecin",
            "Pache il Guaritore",
            "憤血ドクター・パパグ",
            "블러디 로어 군의관 파파고",
            "Znachor Pache",
            "Pache Pacificador",
            "Паче Избавитель",
            "วอร์เมนเดอร์ปาเช",
            "Sıhhi Pache",
            "Huyết Nộ Chiến Bào Pache",
            "血怒戰醫·帕帕古",
            "血怒战医·帕帕古",
            // Historical names
            "Voodoo Priest Pache",
            "باتشي كاهن الفودو",
            "巫毒祭司 · 帕帕古",
            "Voodoo-Priester Pache",
            "Sacerdote Vudú Pache",
            "Prêtre vaudou Pache",
            "Prete Voodoo Pache",
            "좀비 제사장 파파고",
            "Paderi Voodoo Pache",
            "Kapłanka voodoo Pache",
            "Padre Voodoo",
            "Жрец вуду Паче",
            "นักบวชบูดู ปาเช",
            "Vudu Rahibi Pache",
            "Giáo sĩ tà thuật Pache",
            "巫毒祭司 · 帕帕古",
        ],
    },
    Boss {
        id: 30005,
        localized_names: &[
            "سولون بور",
            "Solon Por",
            "ボロック・ソロン",
            "폴락 사우론",
            "Солон Пор",
            "โซลอน ปอร์",
            "Ulusal Lider Por",
            "波洛克 · 索隆",
        ],
    },
];

#[derive(Debug, Default)]
pub struct BossExtractor;

impl BossExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BossExtractor {
    fn section(&self) -> &'static str {
        "boss"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let subtitle = require_string_field(content, "subTitle")?;
        let boss = BOSSES
            .iter()
            .find(|boss| boss.localized_names.iter().any(|name| subtitle.contains(name)))
            .ok_or(ExtractError::InvalidFieldType {
                field: "subTitle",
                expected: "subtitle containing a known GVE alliance boss name",
            })?;

        let mut section = Section::new();
        section.insert("id", Value::from(boss.id));
        Ok(section)
    }
}

#[cfg(test)]
mod tests {
    use rokbattles_mail_sdk::Extractor;
    use serde_json::json;

    use super::*;

    #[test]
    fn identifies_every_boss_in_multiple_languages() {
        let cases = [
            ("Bladefist Andaal Has Been Defeated", 30001),
            ("Bärenwächter Lukor wurde besiegt", 30002),
            ("잔인한 호위군 모도 격파", 30003),
            ("血怒戰醫·帕帕古已被擊敗", 30004),
            ("Ulusal Lider Por yenildi", 30005),
        ];
        for (subtitle, expected_id) in cases {
            let input = json!({"body": {"content": {"subTitle": subtitle}}});
            let section = BossExtractor::new().extract(&input).expect("known boss");
            assert_eq!(section.fields()["id"], json!(expected_id));
        }
    }

    #[test]
    fn identifies_historical_boss_names() {
        let cases = [
            ("\"Fists of Blades\" Andaal Has Been Defeated", 30001),
            ("Смотритель медведей Лукор побежден", 30002),
            ("Shield Chieftain Murdos Has Been Defeated", 30003),
            ("Voodoo Priest Pache Has Been Defeated", 30004),
            ("Solon Por Has Been Defeated", 30005),
        ];
        for (subtitle, expected_id) in cases {
            let input = json!({"body": {"content": {"subTitle": subtitle}}});
            let section = BossExtractor::new().extract(&input).expect("historical boss name");
            assert_eq!(section.fields()["id"], json!(expected_id));
        }
    }

    #[test]
    fn rejects_unknown_boss() {
        let input = json!({"body": {"content": {"subTitle": "Unknown defeated"}}});
        assert!(BossExtractor::new().extract(&input).is_err());
    }
}
