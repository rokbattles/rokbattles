use serde::{Deserialize, Serialize};

/// Processed DuelBattle2 mail output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Mail {
    // id / time / receiver / serverId
    pub metadata: DuelBattle2Metadata,
    // AtkPlayer
    pub sender: DuelBattle2Participant,
    // DefPlayer
    pub opponent: DuelBattle2Participant,
    // AtkPlayer / DefPlayer
    pub results: DuelBattle2Results,
}

/// Metadata extracted from the raw mail sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Metadata {
    // id
    pub email_id: Option<String>,
    // time
    pub email_time: Option<i64>,
    // receiver
    pub email_receiver: Option<String>,
    // serverId
    pub server_id: Option<i64>,
}

/// Sender or opponent details extracted from DuelBattle2 sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Participant {
    // PlayerId
    pub player_id: Option<i64>,
    // PlayerName
    pub player_name: Option<String>,
    // ServerId
    pub kingdom: Option<i64>,
    // Abbr
    pub alliance: Option<String>,
    // DuelTeamId
    pub duel_id: Option<i64>,
    // PlayerAvatar.avatar
    pub avatar_url: Option<String>,
    // PlayerAvatar.avatarFrame
    pub frame_url: Option<String>,
    // MainHero / AssistHero
    pub commanders: DuelBattle2Commanders,
    // Buffs
    pub buffs: Vec<DuelBattle2Buff>,
}

/// Commanders associated with a DuelBattle2 participant.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Commanders {
    // MainHero
    pub primary: Option<DuelBattle2Commander>,
    // AssistHero
    pub secondary: Option<DuelBattle2Commander>,
}

/// Commander details extracted from hero data.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Commander {
    // HeroId
    pub id: Option<i64>,
    // HeroLevel
    pub level: Option<i64>,
    // Star
    pub star: Option<i64>,
    // Awaked
    pub awakened: Option<bool>,
    // Skills
    pub skills: Vec<DuelBattle2Skill>,
}

/// Skill data associated with a commander.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Skill {
    // SkillId
    pub id: Option<i64>,
    // Level
    pub level: Option<i64>,
    // Id
    pub order: Option<i64>,
}

/// Buff data associated with a participant.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Buff {
    // BuffId
    pub id: Option<i64>,
    // BuffValue
    pub value: Option<f64>,
}

/// Aggregated results for DuelBattle2 participants.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Results {
    // KillScore
    pub kill_points: Option<i64>,
    // UnitBadHurt
    pub sev_wounded: Option<i64>,
    // UnitHurt
    pub wounded: Option<i64>,
    // UnitDead
    pub dead: Option<i64>,
    // UnitReturn
    pub heal: Option<i64>,
    // UnitTotal
    pub units: Option<i64>,
    // LosePower
    pub power: Option<i64>,
    // IsWin
    pub win: Option<bool>,
    // KillScore
    pub opponent_kill_points: Option<i64>,
    // UnitBadHurt
    pub opponent_sev_wounded: Option<i64>,
    // UnitHurt
    pub opponent_wounded: Option<i64>,
    // UnitDead
    pub opponent_dead: Option<i64>,
    // UnitReturn
    pub opponent_heal: Option<i64>,
    // UnitTotal
    pub opponent_units: Option<i64>,
    // LosePower
    pub opponent_power: Option<i64>,
    // IsWin
    pub opponent_win: Option<bool>,
}
