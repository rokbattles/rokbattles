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
    pub email_id: String,
    // time
    pub email_time: i64,
    // receiver
    pub email_receiver: String,
    // serverId
    pub server_id: i64,
}

/// Sender or opponent details extracted from DuelBattle2 sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Participant {
    // PlayerId
    pub player_id: i64,
    // PlayerName
    pub player_name: String,
    // ServerId
    pub kingdom: i64,
    // Abbr
    pub alliance: String,
    // DuelTeamId
    pub duel_id: i64,
    // PlayerAvatar.avatar
    pub avatar_url: String,
    // PlayerAvatar.avatarFrame
    pub frame_url: String,
    // MainHero / AssistHero
    pub commanders: DuelBattle2Commanders,
    // Buffs
    pub buffs: Vec<DuelBattle2Buff>,
}

/// Commanders associated with a DuelBattle2 participant.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Commanders {
    // MainHero
    pub primary: DuelBattle2Commander,
    // AssistHero
    pub secondary: DuelBattle2Commander,
}

/// Commander details extracted from hero data.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Commander {
    // HeroId
    pub id: i64,
    // HeroLevel
    pub level: i64,
    // Star
    pub star: i64,
    // Awaked
    pub awakened: bool,
    // Skills
    pub skills: Vec<DuelBattle2Skill>,
}

/// Skill data associated with a commander.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Skill {
    // SkillId
    pub id: i64,
    // Level
    pub level: i64,
    // Id
    pub order: i64,
}

/// Buff data associated with a participant.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Buff {
    // BuffId
    pub id: i64,
    // BuffValue
    pub value: f64,
}

/// Aggregated results for DuelBattle2 participants.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Results {
    // KillScore
    pub kill_points: i64,
    // UnitBadHurt
    pub sev_wounded: i64,
    // UnitHurt
    pub wounded: i64,
    // UnitDead
    pub dead: i64,
    // UnitReturn
    pub heal: i64,
    // UnitTotal
    pub units: i64,
    // LosePower
    pub power: i64,
    // IsWin
    pub win: bool,
    // KillScore
    pub opponent_kill_points: i64,
    // UnitBadHurt
    pub opponent_sev_wounded: i64,
    // UnitHurt
    pub opponent_wounded: i64,
    // UnitDead
    pub opponent_dead: i64,
    // UnitReturn
    pub opponent_heal: i64,
    // UnitTotal
    pub opponent_units: i64,
    // LosePower
    pub opponent_power: i64,
    // IsWin
    pub opponent_win: bool,
}
