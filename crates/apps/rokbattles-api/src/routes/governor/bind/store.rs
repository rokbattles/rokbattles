use futures::TryStreamExt;
use mongodb::bson::{Bson, DateTime, Document, doc, oid::ObjectId};
use rokbattles_bson::bson_to_i64_exact;
use rustc_hash::FxHashSet;

use super::types::ClaimedGovernor;
use crate::{
    db::ReportsStore,
    error::ApiError,
    routes::governor::snapshot::{GovernorSnapshot, find_latest_sender_snapshot},
};

pub(super) const MAX_LILITH_ACCOUNTS: usize = 100;

pub(crate) fn database_error(error: mongodb::error::Error) -> ApiError {
    ApiError::internal(error.to_string())
}

pub(crate) fn primary_filter(discord_id: &str, governor_id: i64) -> Document {
    doc! { "discordId": discord_id, "governorId": governor_id }
}

// Sessions can use any current share. To use a share, an app token must match
// its ID from consent. An empty list permits only primary binds.
fn access_filter(discord_id: &str, authorized_shares: Option<&[ObjectId]>) -> Document {
    let mut share = doc! { "discordId": discord_id };
    if let Some(ids) = authorized_shares {
        share.insert("_id", doc! { "$in": ids });
    }

    doc! { "$or": [
        { "discordId": discord_id },
        { "shares": { "$elemMatch": share } },
    ] }
}

fn is_primary(claim: &Document, discord_id: &str) -> bool {
    claim.get_str("discordId").is_ok_and(|owner| owner == discord_id)
}

pub(crate) fn share_documents(claim: &Document) -> impl Iterator<Item = &Document> {
    claim.get_array("shares").into_iter().flatten().filter_map(Bson::as_document)
}

pub(crate) fn share_for<'a>(claim: &'a Document, discord_id: &str) -> Option<&'a Document> {
    share_documents(claim)
        .find(|share| share.get_str("discordId").is_ok_and(|recipient| recipient == discord_id))
}

fn membership_id(claim: &Document, discord_id: &str) -> Option<ObjectId> {
    if is_primary(claim, discord_id) {
        claim.get_object_id("_id").ok()
    } else {
        share_for(claim, discord_id)?.get_object_id("_id").ok()
    }
}

pub(crate) async fn find_accessible_claim(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
    authorized_shares: Option<&[ObjectId]>,
) -> Result<Document, ApiError> {
    let mut filter = access_filter(discord_id, authorized_shares);
    filter.insert("governorId", governor_id);

    store
        .claimed_governors_collection()
        .find_one(filter)
        .await
        .map_err(database_error)?
        .ok_or_else(|| ApiError::not_found("Claim not found"))
}

pub(super) async fn find_primary_claim(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
) -> Result<Document, ApiError> {
    store
        .claimed_governors_collection()
        .find_one(primary_filter(discord_id, governor_id))
        .await
        .map_err(database_error)?
        .ok_or_else(|| ApiError::not_found("Primary bind not found"))
}

pub(super) async fn existing_primary_bind(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
) -> Result<Option<ClaimedGovernor>, ApiError> {
    let Some(claim) = store
        .claimed_governors_collection()
        .find_one(doc! { "governorId": governor_id })
        .projection(doc! { "discordId": 1 })
        .await
        .map_err(database_error)?
    else {
        return Ok(None);
    };

    if !is_primary(&claim, discord_id) {
        return Err(ApiError::conflict("Governor already claimed"));
    }

    Ok(load_claimed_governors(store, discord_id, None)
        .await?
        .into_iter()
        .find(|claim| claim.governor_id == governor_id && !claim.is_shared))
}

pub(crate) async fn load_claimed_governors(
    store: &ReportsStore,
    discord_id: &str,
    authorized_shares: Option<&[ObjectId]>,
) -> Result<Vec<ClaimedGovernor>, ApiError> {
    list_claims(store, discord_id, authorized_shares).await.map(|(claims, _)| claims)
}

async fn list_claims(
    store: &ReportsStore,
    discord_id: &str,
    authorized_shares: Option<&[ObjectId]>,
) -> Result<(Vec<ClaimedGovernor>, Option<ObjectId>), ApiError> {
    let mut cursor = store
        .claimed_governors_collection()
        .find(access_filter(discord_id, authorized_shares))
        .sort(doc! { "createdAt": -1, "_id": -1 })
        .projection(doc! {
            "discordId": 1,
            "governorId": 1,
            "governorName": 1,
            "governorAvatar": 1,
            "default": 1,
            "shares.discordId": 1,
            "shares._id": 1,
        })
        .await
        .map_err(database_error)?;

    let mut claims = Vec::new();
    let mut memberships = Vec::new();

    while let Some(document) = cursor.try_next().await.map_err(database_error)? {
        let Some(governor_id) = governor_id_from_claim(&document) else {
            continue;
        };
        memberships.push(membership_id(&document, discord_id));
        let is_shared = !is_primary(&document, discord_id);
        claims.push(ClaimedGovernor {
            governor_id,
            governor_name: string_field(&document, "governorName"),
            governor_avatar: string_field(&document, "governorAvatar"),
            default: !is_shared && document.get_bool("default").unwrap_or(false),
            is_shared,
            shared_with: if !is_shared && authorized_shares.is_none() {
                share_documents(&document)
                    .filter_map(|share| string_field(share, "discordId"))
                    .collect()
            } else {
                Vec::new()
            },
        });
    }

    let mut default_to_preserve = None;
    if !claims.is_empty() {
        let preference = store
            .users_collection()
            .find_one(doc! { "discordId": discord_id })
            .projection(doc! { "defaultGovernorMembership": 1 })
            .await
            .map_err(database_error)?;
        let selected = preference
            .as_ref()
            .and_then(|user| user.get_object_id("defaultGovernorMembership").ok());

        let index = default_index(&claims, &memberships, selected);
        default_to_preserve =
            memberships.get(index).copied().flatten().filter(|current| Some(*current) != selected);

        for claim in &mut claims {
            claim.default = false;
        }
        let mut default = claims.remove(index);
        default.default = true;
        claims.insert(0, default);
    }

    Ok((claims, default_to_preserve))
}

fn default_index(
    claims: &[ClaimedGovernor],
    memberships: &[Option<ObjectId>],
    selected: Option<ObjectId>,
) -> usize {
    match selected {
        Some(id) => memberships.iter().position(|membership| *membership == Some(id)),
        None => claims.iter().position(|claim| claim.default),
    }
    .unwrap_or(0)
}

/// Saves the current default before inserting a new bind.
///
/// This keeps the current selection even when it was inferred from the
/// existing binds. Returns whether the account already has a bind.
pub(super) async fn preserve_existing_default(
    store: &ReportsStore,
    discord_id: &str,
) -> Result<bool, ApiError> {
    let (claims, membership) = list_claims(store, discord_id, None).await?;
    if let Some(membership) = membership {
        write_default(store, discord_id, membership).await?;
    }

    Ok(!claims.is_empty())
}

pub(super) async fn set_default(
    store: &ReportsStore,
    discord_id: &str,
    claim: &Document,
) -> Result<(), ApiError> {
    let membership =
        membership_id(claim, discord_id).ok_or_else(|| ApiError::not_found("Claim not found"))?;

    write_default(store, discord_id, membership).await
}

async fn write_default(
    store: &ReportsStore,
    discord_id: &str,
    membership: ObjectId,
) -> Result<(), ApiError> {
    store
        .users_collection()
        .update_one(
            doc! { "discordId": discord_id },
            doc! {
                "$set": { "defaultGovernorMembership": membership }
            },
        )
        .await
        .map_err(database_error)?;

    Ok(())
}

pub(super) fn governor_id_from_claim(claim: &Document) -> Option<i64> {
    claim.get("governorId").and_then(bson_to_i64_exact)
}

fn string_field(document: &Document, key: &str) -> Option<String> {
    document.get_str(key).ok().map(ToOwned::to_owned)
}

pub(super) async fn resolve_claim_app_uid(
    store: &ReportsStore,
    claim: &Document,
) -> Result<Option<i64>, ApiError> {
    if let Some(id) = claim.get("appUid").and_then(bson_to_i64_exact).filter(|id| *id > 0) {
        return Ok(Some(id));
    }

    let Some(governor) = governor_id_from_claim(claim) else { return Ok(None) };
    let uid = find_latest_sender_snapshot(store.battle_collection(), governor)
        .await?
        .and_then(|snapshot| snapshot.app_uid);

    if let (Some(uid), Some(id)) = (uid, claim.get("_id")) {
        // The document ID ties the update to the original bind if the governor is rebound.
        store
            .claimed_governors_collection()
            .update_one(doc! { "_id": id }, doc! { "$set": { "appUid": uid } })
            .await
            .map_err(database_error)?;
    }

    Ok(uid)
}

pub(super) async fn owned_lilith_accounts(
    store: &ReportsStore,
    discord_id: &str,
) -> Result<FxHashSet<i64>, ApiError> {
    let claims = store.claimed_governors_collection();
    let mut ids: FxHashSet<i64> = claims
        .distinct("appUid", doc! { "discordId": discord_id, "appUid": { "$gt": 0 } })
        .await
        .map_err(database_error)?
        .iter()
        .filter_map(bson_to_i64_exact)
        .collect();

    let mut cursor = claims
        .find(doc! { "discordId": discord_id, "appUid": { "$not": { "$gt": 0 } } })
        .projection(doc! { "governorId": 1 })
        .await
        .map_err(database_error)?;

    while let Some(claim) = cursor.try_next().await.map_err(database_error)? {
        if let Some(id) = resolve_claim_app_uid(store, &claim).await? {
            ids.insert(id);
        }
    }

    Ok(ids)
}

pub(super) fn within_lilith_quota(ids: &FxHashSet<i64>, app_uid: i64) -> bool {
    ids.contains(&app_uid) || ids.len() < MAX_LILITH_ACCOUNTS
}

pub(super) async fn insert_claim(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
    snapshot: &GovernorSnapshot,
    default: bool,
) -> mongodb::error::Result<()> {
    store
        .claimed_governors_collection()
        .insert_one(doc! {
            "discordId": discord_id,
            "governorId": governor_id,
            "appUid": snapshot.app_uid,
            "createdAt": DateTime::now(),
            "governorName": snapshot.governor_name.as_deref(),
            "governorAvatar": snapshot.governor_avatar.as_deref(),
            "default": default,
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_the_hundredth_lilith_id() {
        assert!(within_lilith_quota(&(1..100).collect(), 100));
    }

    #[test]
    fn rejects_the_hundred_and_first_lilith_id() {
        assert!(!within_lilith_quota(&(1..=100).collect(), 101));
    }

    #[test]
    fn an_existing_lilith_id_can_bind_another_governor_at_the_limit() {
        assert!(within_lilith_quota(&(1..=100).collect(), 100));
    }

    #[test]
    fn oauth_filter_requires_the_recipient_and_share_id_on_the_same_entry() {
        let allowed = ObjectId::new();
        assert_eq!(
            access_filter("recipient", Some(&[allowed])),
            doc! { "$or": [
                { "discordId": "recipient" },
                { "shares": { "$elemMatch": { "discordId": "recipient", "_id": { "$in": [allowed] } } } },
            ] },
        );
    }

    #[test]
    fn resolves_owned_and_shared_defaults_by_the_current_membership() {
        let owner = ObjectId::new();
        let share = ObjectId::new();
        let document = doc! { "_id": owner, "discordId": "primary", "shares": [
            { "_id": share, "discordId": "recipient" },
        ] };
        assert_eq!(membership_id(&document, "primary"), Some(owner));
        assert_eq!(membership_id(&document, "recipient"), Some(share));
        assert_eq!(membership_id(&document, "stranger"), None);
        assert_eq!(default_index(&[], &[Some(owner), Some(share)], Some(share)), 1);
        assert_eq!(default_index(&[], &[Some(owner), Some(share)], Some(ObjectId::new())), 0);
    }
}
