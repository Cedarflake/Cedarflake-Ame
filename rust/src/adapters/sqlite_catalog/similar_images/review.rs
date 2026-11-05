use crate::domain::ScanError;
use crate::domain::similar_images::SimilarContentKey;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarReviewPage, SimilarReviewRange, SimilarUndoResult,
    SimilarityDegree,
};
use crate::ports::similar_images::review::{SimilarMemberCandidates, SimilarReviewRepository};

use super::super::{SqliteCatalog, database_error};

mod decisions;
mod projection;
mod queries;

impl SimilarReviewRepository for SqliteCatalog {
    fn load_similar_groups(
        &self,
        degree: SimilarityDegree,
        ignored: bool,
        after: &str,
    ) -> Result<SimilarReviewPage, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let total = queries::count_groups(&transaction, degree, ignored)?;
        let page = queries::groups(&transaction, degree, ignored, after, 0, total)?;
        transaction.commit().map_err(database_error)?;
        Ok(page)
    }

    fn seek_similar_groups(
        &self,
        degree: SimilarityDegree,
        ignored: bool,
        index: u64,
    ) -> Result<SimilarReviewRange, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let total = queries::count_groups(&transaction, degree, ignored)?;
        let page_size = crate::domain::similar_images::review::SIMILARITY_REVIEW_PAGE_SIZE as u64;
        let start_index = index.min(total.saturating_sub(1)) / page_size * page_size;
        let page = queries::groups(&transaction, degree, ignored, "", start_index, total)?;
        transaction.commit().map_err(database_error)?;
        Ok(SimilarReviewRange { start_index, page })
    }

    fn load_similar_members(
        &self,
        group_id: &str,
        degree: SimilarityDegree,
        after: Option<&SimilarContentKey>,
    ) -> Result<SimilarMemberCandidates, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let page = queries::members(&transaction, group_id, degree, after)?;
        transaction.commit().map_err(database_error)?;
        Ok(page)
    }

    fn prefer_similar_member(
        &mut self,
        context: &SimilarReviewContext,
        selection: &SimilarMemberSelection,
        preferred: bool,
    ) -> Result<SimilarGroupDecision, ScanError> {
        let transaction = self.begin_write()?;
        let decision = decisions::prefer(&transaction, context, selection, preferred)?;
        transaction.commit().map_err(database_error)?;
        Ok(decision)
    }

    fn ignore_similar_group(
        &mut self,
        context: &SimilarReviewContext,
        ignored: bool,
    ) -> Result<SimilarGroupDecision, ScanError> {
        let transaction = self.begin_write()?;
        let decision = decisions::ignore(&transaction, context, ignored)?;
        transaction.commit().map_err(database_error)?;
        Ok(decision)
    }

    fn exclude_similar_member(
        &mut self,
        context: &SimilarReviewContext,
        selection: &SimilarMemberSelection,
        exclusion_id: &str,
    ) -> Result<SimilarExclusionResult, ScanError> {
        let transaction = self.begin_write()?;
        let result = decisions::exclude(&transaction, context, selection, exclusion_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn undo_similar_exclusion(
        &mut self,
        exclusion: &SimilarMemberExclusion,
    ) -> Result<SimilarUndoResult, ScanError> {
        let transaction = self.begin_write()?;
        let result = decisions::undo(&transaction, exclusion)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
