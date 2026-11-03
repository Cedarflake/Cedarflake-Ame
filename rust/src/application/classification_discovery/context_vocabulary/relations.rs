use std::collections::{BTreeSet, VecDeque};

use crate::domain::classification::maintenance::CategoryMergeContext;

use super::*;

impl NamingContextCatalog {
    pub(crate) fn broader_merge_context(
        &self,
        narrower: ConceptId,
        broader: ConceptId,
        cancelled: &AtomicBool,
    ) -> Result<Option<CategoryMergeContext>, ScanError> {
        check_cancelled(cancelled)?;
        let (Some(narrower_concept), Some(broader_concept)) =
            (self.contexts.get(&narrower), self.contexts.get(&broader))
        else {
            return Ok(None);
        };
        if narrower == broader {
            return Ok(None);
        }
        let Some(path) = self.broader_path(narrower, broader, cancelled)? else {
            return Ok(None);
        };
        // Cycles establish no strict semantic containment, even beyond the admitted path.
        if self.reaches(broader, narrower, cancelled)? {
            return Ok(None);
        }
        Ok(Some(CategoryMergeContext {
            catalog: self.revision,
            narrower: narrower_concept.clone(),
            broader: broader_concept.clone(),
            path,
        }))
    }

    fn broader_path(
        &self,
        from: ConceptId,
        target: ConceptId,
        cancelled: &AtomicBool,
    ) -> Result<Option<Vec<ConceptId>>, ScanError> {
        let mut pending = VecDeque::from([vec![from]]);
        let mut visited = BTreeSet::from([from]);
        while let Some(path) = pending.pop_front() {
            check_cancelled(cancelled)?;
            if path.len() > 4 {
                continue;
            }
            for &parent in self
                .broader
                .get(path.last().expect("a path is nonempty"))
                .into_iter()
                .flatten()
            {
                if !visited.insert(parent) {
                    continue;
                }
                let mut extended = path.clone();
                extended.push(parent);
                if parent == target {
                    return Ok(Some(extended));
                }
                pending.push_back(extended);
            }
        }
        Ok(None)
    }

    fn reaches(
        &self,
        from: ConceptId,
        target: ConceptId,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        let mut pending = VecDeque::from([from]);
        let mut visited = BTreeSet::from([from]);
        while let Some(current) = pending.pop_front() {
            check_cancelled(cancelled)?;
            for &parent in self.broader.get(&current).into_iter().flatten() {
                if parent == target {
                    return Ok(true);
                }
                if visited.insert(parent) {
                    pending.push_back(parent);
                }
            }
        }
        Ok(false)
    }
}
