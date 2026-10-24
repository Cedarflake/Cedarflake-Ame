use crate::domain::organization::{OrganizationDisposition, OrganizationGroupDecision};

use super::*;

impl SqliteOrganizationBatch {
    pub(super) fn prepare_inner(
        &mut self,
        refresh: bool,
    ) -> Result<OrganizationBatchSummary, ScanError> {
        self.control.check()?;
        if refresh {
            if self.prepared.is_none()
                || self.control.snapshot().phase != OrganizationBatchPhase::Ready
            {
                return Err(stale_selection());
            }
        } else if let Some(prepared) = &self.prepared {
            return Ok(prepared.summary.clone());
        }
        self.control.report(OrganizationBatchPhase::Preparing, 0, 0);
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let (catalog_revision, organization_revision) = revisions(&transaction)?;
        if !refresh {
            if catalog_revision != self.catalog_revision {
                return Err(stale_selection());
            }
            staging::materialize(&transaction, &self.query, self.all_matching)?;
        }
        let group_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM temp.organization_batch_groups",
                [],
                |row| read_unsigned(row, 0),
            )
            .map_err(database_error)?;
        if group_count == 0 {
            return Err(ScanError::new(
                "organization_selection_empty",
                "No current image groups are selected",
            ));
        }
        let mut summary = OrganizationBatchSummary {
            catalog_revision,
            organization_revision,
            group_count,
            favorite_count: 0,
            assigned_count: 0,
            conflicting_count: 0,
            common_album: None,
        };
        let mut hasher = blake3::Hasher::new();
        let mut decisions = blake3::Hasher::new();
        authority::field(&mut hasher, b"ame-organization-batch-v1");
        authority::field(&mut decisions, b"ame-organization-batch-decisions-v1");
        for value in [catalog_revision, organization_revision, group_count] {
            authority::field(&mut hasher, &value.to_le_bytes());
        }
        let mut after = 0;
        let mut completed = 0;
        loop {
            let groups = staging::groups(&transaction, after)?;
            if groups.is_empty() {
                break;
            }
            for group in groups {
                self.control.check()?;
                let target = selection::target(&transaction, &group.key)?;
                if refresh && group.target.as_ref() != Some(&target) {
                    return Err(stale_selection());
                }
                let inspected = inspection::inspect_group(&transaction, &group.key)?;
                let decision = inspected.decision;
                if completed == 0 {
                    summary.common_album = decision.album.clone();
                } else if summary.common_album.as_ref().map(|album| &album.album_id)
                    != decision.album.as_ref().map(|album| &album.album_id)
                {
                    summary.common_album = None;
                }
                summary.favorite_count += u64::from(decision.favorite);
                summary.assigned_count += u64::from(decision.album.is_some());
                summary.conflicting_count += u64::from(decision.has_conflict);
                authority::group_fields(&mut hasher, &target);
                decision_fields(&mut decisions, &decision);
                if let Some(version) = inspected.version {
                    authority::field(&mut decisions, version.record_id.as_bytes());
                    authority::field(&mut decisions, &version.organization_revision.to_le_bytes());
                    authority::field(&mut decisions, &version.favorite_revision.to_le_bytes());
                } else {
                    authority::field(&mut decisions, b"unhandled");
                }
                if !refresh {
                    staging::retain_target(&transaction, group.sequence, &target)?;
                }
                after = group.sequence;
                completed += 1;
                self.control
                    .report(OrganizationBatchPhase::Preparing, completed, group_count);
            }
        }
        let decisions = decisions.finalize().to_hex().to_string();
        if refresh
            && self
                .prepared
                .as_ref()
                .is_none_or(|previous| previous.decisions != decisions)
        {
            return Err(stale_selection());
        }
        self.control.check()?;
        transaction.commit().map_err(database_error)?;
        self.prepared = Some(PreparedBatch {
            summary: summary.clone(),
            authority: hasher.finalize().to_hex().to_string(),
            decisions,
        });
        self.control
            .report(OrganizationBatchPhase::Ready, group_count, group_count);
        Ok(summary)
    }
}

fn decision_fields(hasher: &mut blake3::Hasher, decision: &OrganizationGroupDecision) {
    let disposition = match decision.disposition {
        OrganizationDisposition::Pending => "pending",
        OrganizationDisposition::Deferred => "deferred",
        OrganizationDisposition::Organized => "organized",
        OrganizationDisposition::Ignored => "ignored",
    };
    authority::field(
        hasher,
        &[u8::from(decision.favorite), u8::from(decision.has_conflict)],
    );
    authority::field(hasher, disposition.as_bytes());
    authority::field(
        hasher,
        decision
            .album
            .as_ref()
            .map_or("", |album| &album.album_id)
            .as_bytes(),
    );
}
