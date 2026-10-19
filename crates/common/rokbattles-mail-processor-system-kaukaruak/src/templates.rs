pub(crate) struct Locale {
    pub subtitle: &'static str,
    pub difficulties: [&'static str; 5],
}

pub(crate) const DIFFICULTIES: [&str; 5] = ["easy", "normal", "hard", "nightmare", "hell"];

pub(crate) const LOCALES: &[Locale] = &[
    // ar
    Locale {
        subtitle: "اكتمل التحدي من المستوى {p2} عند مستوى صعوبة {p1}",
        difficulties: ["سهل", "عادي", "صعب", "كابوس", "جحيم"],
    },
    // de
    Locale {
        subtitle: "Lv.-{p2}-Prüfung auf Schwierigkeitsgrad {p1} abgeschlossen",
        difficulties: ["EINFACH", "NORMAL", "SCHWER", "ALBTRAUM", "HÖLLISCH"],
    },
    // en
    Locale {
        subtitle: "Lvl {p2} Trial Completed At {p1} Difficulty",
        difficulties: ["EASY", "NORMAL", "HARD", "NIGHTMARE", "HELL"],
    },
    // es
    Locale {
        subtitle: "Prueba de nivel {p2} completada en la dificultad {p1}",
        difficulties: ["FACIL", "NORMAL", "DIFICIL", "PESADILLA", "INFIERNO"],
    },
    // fr
    Locale {
        subtitle: "Épreuve de niv. {p2} complétée à la difficulté {p1}",
        difficulties: ["FACILE", "NORMAL", "DIFFICILE", "EFFROYABLE", "DÉMONIAQUE"],
    },
    // id
    Locale {
        subtitle: "Ujian Lvl. {p2} Diselesaikan Di Kesulitan {p1}",
        difficulties: ["EASY", "NORMAL", "HARD", "NIGHTMARE", "HELL"],
    },
    // it
    Locale {
        subtitle: "Completato liv {p2} della prova alla difficoltà {p1}",
        difficulties: ["FACILE", "NORMALE", "DIFFICILE", "INCUBO", "INFERNO"],
    },
    // ja
    Locale {
        subtitle: "{p1}難易度レベル{p2}の試練をクリアしました",
        difficulties: ["イージー", "ノーマル", "ハード", "ナイトメア", "ヘル"],
    },
    // ko
    Locale {
        subtitle: "이미 {p1} 난이도 {p2}레벨의 시련을 완료했습니다.",
        difficulties: ["쉬움", "일반", "어려움", "매우 어려움", "지옥"],
    },
    // ms
    Locale {
        subtitle: "Percubaan Thp {p2} Selesai Pada Kesukaran {p1}",
        difficulties: ["MUDAH", "BIASA", "SUKAR", "MIMPI NGERI", "NERAKA"],
    },
    // pl
    Locale {
        subtitle: "Ukończono próbę poz. {p2} na poziomie trudności: {p1}",
        difficulties: ["Łatwy", "Normalny", "Trudny", "Koszmar", "Piekło"],
    },
    // pt
    Locale {
        subtitle: "Teste de nv. {p2} Concluído na Dificuldade {p1}",
        difficulties: ["FÁCIL", "NORMAL", "DIFÍCIL", "PESADELO", "INFERNO"],
    },
    // ru
    Locale {
        subtitle: "Испытание {p2} ур. пройдено на сложности «{p1}»",
        difficulties: ["ЛЕГКО", "НОРМАЛЬНО", "ТЯЖЕЛО", "КОШМАР", "АД"],
    },
    // th
    Locale {
        subtitle: "บททดสอบ เลเวล {p2} เสร็จสิ้นที่ระดับความยาก {p1}",
        difficulties: ["ง่าย", "ปกติ", "ยาก", "ฝันร้าย", "นรก"],
    },
    // tr
    Locale {
        subtitle: "{p2}. Sv. Sınav {p1} Seviyesinde Tamamlandı",
        difficulties: ["KOLAY", "NORMAL", "ZOR", "KABUS", "CEHENNEM"],
    },
    // vi
    Locale {
        subtitle: "Đã Hoàn Tất Thử Thách Cấp {p2} Ở Độ Khó {p1}",
        difficulties: ["DỄ", "BÌNH THƯỜNG", "KHÓ", "ÁC MỘNG", "ĐỊA NGỤC"],
    },
    // zh_CN
    Locale {
        subtitle: "您已经完成了{p1}难度等级{p2}的试炼",
        difficulties: ["简单", "普通", "困难", "噩梦", "地狱"],
    },
    // zh_TW
    Locale {
        subtitle: "您已經完成了{p1}難度等級{p2}的試煉",
        difficulties: ["簡單", "普通", "困難", "惡夢", "地獄"],
    },
];
