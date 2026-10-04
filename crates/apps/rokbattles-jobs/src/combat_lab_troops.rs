//! Troop eligibility from each fight's casualties and the opposing side's kill points.

use mongodb::bson::{Document, doc};

const T4_KILL_POINTS: i64 = 10;

/// Require a T4-or-better casualty average on both sides, allowing mixed troop tiers.
pub(crate) fn eligible_troop_kp_expr() -> Document {
    doc! {
        "$and": [
            side_meets_t4_kp_expr("sender", "opponent"),
            side_meets_t4_kp_expr("opponent", "sender"),
        ]
    }
}

fn side_meets_t4_kp_expr(side: &str, enemy: &str) -> Document {
    // KP counts severely wounded and dead troops. Light wounds, healing, and changes
    // in remaining units do not belong in this per-opponent casualty total.
    let wounded = format!("$opponents.battle_results.{side}.severely_wounded");
    let dead = format!("$opponents.battle_results.{side}.dead");
    let kill_points = format!("$opponents.battle_results.{enemy}.kill_points");
    doc! {
        "$cond": [
            {
                "$and": [
                    { "$isNumber": &wounded },
                    { "$isNumber": &dead },
                    { "$isNumber": &kill_points },
                    { "$gte": [&wounded, 0_i64] },
                    { "$gte": [&dead, 0_i64] },
                    { "$gte": [&kill_points, 0_i64] },
                ]
            },
            {
                "$gte": [
                    kill_points,
                    { "$multiply": [T4_KILL_POINTS, { "$add": [wounded, dead] }] },
                ]
            },
            // Missing results cannot establish troop tier. Zero casualties also pass.
            true,
        ]
    }
}
