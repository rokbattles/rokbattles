use std::num::NonZeroUsize;

use crate::{
    auth::{BoundSession, Credentials},
    client::RokGtClient,
    error::RokGtError,
    models::{KingdomMember, KingdomMemberBatch, MemberDateRange},
};

/// Pulls kingdom members in fixed-size batches.
#[derive(Debug)]
pub struct KingdomMemberBatcher<'a> {
    pub(crate) client: &'a RokGtClient,
    pub(crate) session: BoundSession,
    pub(crate) credentials: Option<Credentials>,
    pub(crate) server_ids: Vec<u32>,
    pub(crate) next_index: usize,
    pub(crate) batch_size: NonZeroUsize,
    pub(crate) range: MemberDateRange,
}

impl KingdomMemberBatcher<'_> {
    /// Kingdoms selected for this run.
    pub fn server_ids(&self) -> &[u32] {
        &self.server_ids
    }

    /// Whether another batch is available.
    pub fn has_next(&self) -> bool {
        self.next_index < self.server_ids.len()
    }

    /// Fetch the next batch, or `None` after the last kingdom.
    pub async fn next_batch(&mut self) -> Result<Option<KingdomMemberBatch>, RokGtError> {
        if !self.has_next() {
            return Ok(None);
        }

        let end = self.next_index.saturating_add(self.batch_size.get()).min(self.server_ids.len());
        let batch_server_ids = self.server_ids[self.next_index..end].to_vec();
        self.next_index = end;

        let mut members = Vec::new();
        for server_id in &batch_server_ids {
            let mut server_members = self.fetch_kingdom_members_with_retry(*server_id).await?;
            members.append(&mut server_members);
        }

        Ok(Some(KingdomMemberBatch { server_ids: batch_server_ids, members }))
    }

    async fn fetch_kingdom_members_with_retry(
        &mut self,
        server_id: u32,
    ) -> Result<Vec<KingdomMember>, RokGtError> {
        match self.client.fetch_kingdom_members(&self.session, server_id, &self.range).await {
            Ok(members) => Ok(members),
            Err(error) if error.is_auth_failure() && self.credentials.is_some() => {
                let credentials = self.credentials.clone().ok_or(error)?;
                self.session = self.client.authenticate_and_bind_default_role(&credentials).await?;
                self.client.fetch_kingdom_members(&self.session, server_id, &self.range).await
            }
            Err(error) => Err(error),
        }
    }
}
