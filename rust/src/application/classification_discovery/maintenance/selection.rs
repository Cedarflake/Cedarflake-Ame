use std::collections::{BTreeMap, BTreeSet};

use super::super::scheme::{ScoredSample, selection};
use super::super::{CategoryAdmissionCandidate, MemberAdmission};
use super::*;

pub(super) struct Selection {
    pub categories: Vec<CategoryDefinition>,
    pub candidates: Vec<CategoryExpansionCandidate>,
    pub outcomes: Vec<CategorySchemeCandidate>,
}

pub(super) fn proposals(
    admission: &CategoryAdmission,
    discovery: &ConceptConsolidation,
) -> Result<Vec<CategoryDefinition>, ScanError> {
    let mut proposed = Vec::new();
    for candidate in &admission.candidates {
        if candidate.disposition != CategoryAdmissionDisposition::Proposed {
            continue;
        }
        let concept = &discovery
            .candidates
            .iter()
            .find(|entry| entry.candidate.concept.id() == candidate.concept_id)
            .ok_or_else(stale)?
            .candidate
            .concept;
        proposed.push(
            CategoryDefinition::new(
                managed_category_id(concept.id(), &discovery.naming)?,
                concept.label().to_owned(),
                vec![concept.description().to_owned()],
            )
            .map_err(|_| stale())?,
        );
    }
    proposed.sort_by(|left, right| left.id().cmp(right.id()));
    Ok(proposed)
}

pub(super) fn select(
    admission: &CategoryAdmission,
    proposed: &[CategoryDefinition],
    existing: &Taxonomy,
    capacity: &CategoryExpansionCapacity,
    policy: &CategoryAdmissionPolicy,
    cancelled: &AtomicBool,
) -> Result<Selection, ScanError> {
    let admitted: BTreeMap<_, _> = admission
        .candidates
        .iter()
        .map(|candidate| {
            Ok((
                managed_category_id(candidate.concept_id, &admission.naming)?,
                candidate,
            ))
        })
        .collect::<Result<_, ScanError>>()?;
    let candidates: Vec<&CategoryAdmissionCandidate> = proposed
        .iter()
        .map(|category| admitted.get(category.id()).copied().ok_or_else(stale))
        .collect::<Result<_, _>>()?;
    let first = candidates.first().ok_or_else(stale)?;
    let mut samples = Vec::with_capacity(first.members.len());
    for (index, member) in first.members.iter().enumerate() {
        check_cancelled(cancelled)?;
        let mut scores = Vec::with_capacity(candidates.len());
        let mut eligible = Vec::with_capacity(candidates.len());
        for candidate in &candidates {
            let compared = candidate.members.get(index).ok_or_else(stale)?;
            if compared.content != member.content || compared.current != member.current {
                return Err(stale());
            }
            scores.push(compared.similarity);
            eligible.push(compared.disposition == MemberAdmission::Supports);
        }
        samples.push(ScoredSample {
            content: member.content.clone(),
            baseline: member.current.clone(),
            proposed: scores,
            eligible,
        });
    }
    let mut criteria = policy.criteria();
    criteria.max_proposals = criteria
        .max_proposals
        .min(
            capacity
                .maximum_generated
                .saturating_sub(capacity.active_generated.len()),
        )
        .min(256usize.saturating_sub(existing.categories().len()));
    let definitions: Vec<_> = proposed.iter().collect();
    let outcomes = selection::select(
        existing.categories(),
        &definitions,
        &samples,
        criteria,
        cancelled,
    )?;
    let selected: BTreeSet<_> = outcomes
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.disposition == CategorySchemeDisposition::Admitted)
        .map(|(index, _)| index)
        .collect();
    let mut supports = vec![Vec::new(); proposed.len()];
    let mut improvements = vec![0.0f64; proposed.len()];
    for sample in &samples {
        check_cancelled(cancelled)?;
        if let Some((index, _)) = selection::supporting_candidate(&selected, sample, criteria) {
            supports[index].push(sample.content.clone());
            improvements[index] += f64::from(sample.proposed[index] - sample.baseline.similarity);
        }
    }
    let mut selected_categories = Vec::with_capacity(selected.len());
    let mut selected_candidates = Vec::with_capacity(selected.len());
    for index in selected {
        let members = std::mem::take(&mut supports[index]);
        if members.len() < criteria.minimum_members {
            return Err(stale());
        }
        selected_candidates.push(CategoryExpansionCandidate {
            category_id: proposed[index].id().clone(),
            concept_id: candidates[index].concept_id,
            mean_supporting_improvement: (improvements[index] / members.len() as f64) as f32,
            supporting_contents: members,
        });
        selected_categories.push(proposed[index].clone());
    }
    Ok(Selection {
        categories: selected_categories,
        candidates: selected_candidates,
        outcomes,
    })
}
