use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::domain::classification::discovery::ConceptId;

#[derive(Clone, Copy, Default)]
pub(super) struct MemberCoverage([u64; 8]);

impl MemberCoverage {
    pub fn insert(&mut self, index: usize) {
        self.0[index / 64] |= 1 << (index % 64);
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }

    fn contains(&self, other: Self) -> bool {
        self.0
            .iter()
            .zip(other.0)
            .all(|(&word, required)| word & required == required)
    }

    fn uncovered(&self, covered: Self) -> usize {
        self.0
            .iter()
            .zip(covered.0)
            .map(|(&word, covered)| (word & !covered).count_ones() as usize)
            .sum()
    }

    fn include(&mut self, other: Self) {
        for (word, additional) in self.0.iter_mut().zip(other.0) {
            *word |= additional;
        }
    }
}

pub(super) struct ContextSupport {
    pub id: ConceptId,
    pub coverage: MemberCoverage,
    pub mean_improvement: f32,
    pub contrast: f32,
}

pub(super) struct ContextSelection {
    pub selected: Vec<ConceptId>,
    pub subsumed: usize,
    pub candidate_limit_reached: bool,
}

pub(super) fn select(
    contexts: Vec<ContextSupport>,
    parents: &BTreeMap<ConceptId, Vec<ConceptId>>,
    minimum_new_members: usize,
    max_candidates: usize,
) -> ContextSelection {
    let coverage: BTreeMap<_, _> = contexts
        .iter()
        .map(|context| (context.id, context.coverage))
        .collect();
    let mut children = BTreeMap::<_, Vec<_>>::new();
    let mut has_broader = BTreeSet::new();
    for context in &contexts {
        for parent in parents.get(&context.id).into_iter().flatten() {
            if *parent != context.id
                && coverage
                    .get(parent)
                    .is_some_and(|parent_coverage| parent_coverage.contains(context.coverage))
            {
                has_broader.insert(context.id);
                children.entry(*parent).or_default().push(context.id);
            }
        }
    }
    let roots: BTreeSet<_> = coverage
        .keys()
        .filter(|id| !has_broader.contains(*id))
        .copied()
        .collect();
    let mut pending: VecDeque<_> = roots.iter().copied().collect();
    let mut reached = roots.clone();
    while let Some(parent) = pending.pop_front() {
        for child in children.get(&parent).into_iter().flatten() {
            if reached.insert(*child) {
                pending.push_back(*child);
            }
        }
    }
    let subsumed: BTreeSet<_> = reached.difference(&roots).copied().collect();
    let mut remaining: Vec<_> = contexts
        .into_iter()
        .filter(|context| !subsumed.contains(&context.id))
        .collect();
    let mut covered = MemberCoverage::default();
    let mut selected = Vec::new();
    while selected.len() < max_candidates {
        let Some((index, next)) = remaining
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.coverage
                    .uncovered(covered)
                    .cmp(&right.coverage.uncovered(covered))
                    .then_with(|| left.mean_improvement.total_cmp(&right.mean_improvement))
                    .then_with(|| left.contrast.total_cmp(&right.contrast))
                    .then_with(|| right.id.cmp(&left.id))
            })
        else {
            break;
        };
        if next.coverage.uncovered(covered) < minimum_new_members {
            break;
        }
        covered.include(next.coverage);
        selected.push(next.id);
        remaining.swap_remove(index);
    }
    ContextSelection {
        selected,
        subsumed: subsumed.len(),
        candidate_limit_reached: remaining
            .iter()
            .any(|context| context.coverage.uncovered(covered) >= minimum_new_members),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(id: u8, members: &[usize], improvement: f32) -> ContextSupport {
        let mut coverage = MemberCoverage::default();
        for &index in members {
            coverage.insert(index);
        }
        ContextSupport {
            id: ConceptId::from_bytes([id; 32]),
            coverage,
            mean_improvement: improvement,
            contrast: 0.5,
        }
    }

    #[test]
    fn broader_context_keeps_coverage_and_independent_members_keep_a_second_context() {
        let contexts = vec![
            context(1, &[0, 1], 0.9),
            context(2, &[0, 1, 2], 0.1),
            context(3, &[3, 4], 0.2),
        ];
        let parents = BTreeMap::from([(contexts[0].id, vec![contexts[1].id])]);
        let result = select(contexts, &parents, 2, 64);
        assert_eq!(
            result.selected,
            vec![
                ConceptId::from_bytes([2; 32]),
                ConceptId::from_bytes([3; 32])
            ]
        );
        assert_eq!(result.subsumed, 1);
        assert!(!result.candidate_limit_reached);
    }

    #[test]
    fn cycles_and_redundant_names_do_not_erase_every_context_or_multiply_categories() {
        let contexts = vec![
            context(1, &[0, 1], 0.2),
            context(2, &[0, 1], 0.3),
            context(3, &[0, 1], 0.1),
        ];
        let parents = BTreeMap::from([
            (contexts[0].id, vec![contexts[1].id]),
            (contexts[1].id, vec![contexts[0].id]),
        ]);
        let result = select(contexts, &parents, 2, 64);
        assert_eq!(result.selected, vec![ConceptId::from_bytes([2; 32])]);
        assert_eq!(result.subsumed, 0);
        assert!(!result.candidate_limit_reached);
    }
}
