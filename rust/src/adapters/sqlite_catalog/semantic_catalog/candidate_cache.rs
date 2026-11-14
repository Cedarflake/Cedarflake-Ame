use std::mem::size_of;
use std::sync::{Arc, Mutex};

use crate::domain::GalleryQuery;
use crate::ports::semantic_catalog::SemanticCandidate;

const RETAINED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub(in super::super) struct CandidateCache {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    revision: u64,
    entry: Option<Entry>,
}

struct Entry {
    query: GalleryQuery,
    candidates: Arc<Vec<SemanticCandidate>>,
}

pub(super) struct Capture {
    candidates: Option<Vec<SemanticCandidate>>,
    bytes: usize,
}

impl CandidateCache {
    pub(super) fn get(
        &self,
        query: &GalleryQuery,
        revision: u64,
    ) -> Option<Arc<Vec<SemanticCandidate>>> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.advance(revision);
        state
            .entry
            .as_ref()
            .filter(|entry| state.revision == revision && entry.query == *query)
            .map(|entry| Arc::clone(&entry.candidates))
    }

    pub(super) fn publish(&self, query: &GalleryQuery, revision: u64, capture: Capture) {
        let Some(candidates) = capture.candidates else {
            return;
        };
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.advance(revision);
        if state.revision == revision {
            state.entry = Some(Entry {
                query: query.clone(),
                candidates: Arc::new(candidates),
            });
        }
    }
}

impl State {
    fn advance(&mut self, revision: u64) {
        if revision > self.revision {
            self.revision = revision;
            self.entry = None;
        }
    }
}

impl Capture {
    pub(super) fn new(query: &GalleryQuery) -> Self {
        let bytes = size_of::<Entry>()
            + size_of::<Vec<SemanticCandidate>>()
            + 2 * size_of::<usize>()
            + super::super::gallery_projection_cache::query_bytes(query);
        Self {
            candidates: (bytes <= RETAINED_BYTES).then(Vec::new),
            bytes,
        }
    }

    pub(super) fn append(&mut self, window: &[SemanticCandidate]) {
        let Some(candidates) = self.candidates.as_mut() else {
            return;
        };
        self.bytes += window
            .iter()
            .map(|candidate| {
                candidate.source.asset_id.len()
                    + candidate.source.location_id.len()
                    + candidate
                        .content
                        .as_ref()
                        .map_or(0, |content| content.scheme().len())
            })
            .sum::<usize>();
        let count = candidates.len().saturating_add(window.len());
        if self.bytes + count * size_of::<SemanticCandidate>() > RETAINED_BYTES {
            self.candidates = None;
            return;
        }
        if candidates.capacity() < count {
            let capacity = (candidates.capacity() * 2)
                .max(count)
                .min((RETAINED_BYTES - self.bytes) / size_of::<SemanticCandidate>());
            candidates.reserve_exact(capacity - candidates.len());
        }
        if self.bytes + candidates.capacity() * size_of::<SemanticCandidate>() > RETAINED_BYTES {
            self.candidates = None;
            return;
        }
        candidates.extend_from_slice(window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::organization::OrganizationSelection;

    #[test]
    fn oversized_and_older_selections_cannot_replace_current_candidates() {
        let query = GalleryQuery::default();
        let candidate = SemanticCandidate {
            source: OrganizationSelection {
                asset_id: "a".repeat(64),
                location_id: "b".repeat(64),
                source_generation: 1,
            },
            content: None,
        };
        let cache = CandidateCache::default();
        let mut current = Capture::new(&query);
        current.append(std::slice::from_ref(&candidate));
        cache.publish(&query, 2, current);
        cache.publish(&query, 1, Capture::new(&query));
        assert_eq!(cache.get(&query, 2).unwrap().len(), 1);
        assert!(cache.get(&query, 3).is_none());
        cache.publish(&query, 2, Capture::new(&query));
        assert!(cache.get(&query, 3).is_none());

        let mut oversized = Capture::new(&query);
        let window = vec![candidate; 256];
        for _ in 0..1024 {
            oversized.append(&window);
            if oversized.candidates.is_none() {
                break;
            }
        }
        assert!(oversized.candidates.is_none());
        cache.publish(&query, 3, oversized);
        assert!(cache.get(&query, 3).is_none());
    }
}
