use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use super::super::{CategoryAdmissionCriteria, check_cancelled};
use super::{
    CategoryDefinition, CategorySchemeCandidate, CategorySchemeDisposition, ScanError, ScoredSample,
};

#[derive(Clone, Copy, Default)]
struct Support {
    members: usize,
    gain: f64,
}

pub(in crate::application::classification_discovery) fn select(
    existing: &[CategoryDefinition],
    proposed: &[&CategoryDefinition],
    samples: &[ScoredSample],
    criteria: CategoryAdmissionCriteria,
    cancelled: &AtomicBool,
) -> Result<Vec<CategorySchemeCandidate>, ScanError> {
    let mut active = BTreeSet::new();
    let mut outcomes = Vec::with_capacity(proposed.len());
    for (index, category) in proposed.iter().enumerate() {
        let disposition = if let Some(known) = existing
            .iter()
            .find(|known| known.descriptions() == category.descriptions())
        {
            CategorySchemeDisposition::ExistingDefinition(known.id().clone())
        } else if existing
            .iter()
            .any(|known| known.label().to_lowercase() == category.label().to_lowercase())
        {
            CategorySchemeDisposition::NameConflict
        } else {
            active.insert(index);
            CategorySchemeDisposition::Admitted
        };
        outcomes.push(CategorySchemeCandidate {
            category_id: category.id().clone(),
            disposition,
            unique_members: 0,
            mean_marginal_gain: 0.0,
        });
    }
    loop {
        check_cancelled(cancelled)?;
        let statistics = support(&active, proposed.len(), samples, criteria);
        let weak: Vec<_> = active
            .iter()
            .copied()
            .filter(|index| statistics[*index].members < criteria.minimum_members)
            .collect();
        let (remove, disposition) = if !weak.is_empty() {
            (
                weakest(&weak, &statistics),
                CategorySchemeDisposition::OverlappingProposal,
            )
        } else {
            let duplicate_names: Vec<_> = active
                .iter()
                .copied()
                .filter(|index| {
                    active.iter().any(|other| {
                        index != other
                            && proposed[*index].label().to_lowercase()
                                == proposed[*other].label().to_lowercase()
                    })
                })
                .collect();
            if !duplicate_names.is_empty() {
                (
                    weakest(&duplicate_names, &statistics),
                    CategorySchemeDisposition::NameConflict,
                )
            } else if active.len() > criteria.max_proposals {
                (
                    weakest(&active.iter().copied().collect::<Vec<_>>(), &statistics),
                    CategorySchemeDisposition::AdditionBudget,
                )
            } else {
                for index in active {
                    record(&mut outcomes[index], statistics[index]);
                }
                break;
            }
        };
        let index = remove.ok_or_else(super::invalid)?;
        record(&mut outcomes[index], statistics[index]);
        outcomes[index].disposition = disposition;
        if outcomes[index].disposition == CategorySchemeDisposition::OverlappingProposal
            && support(&BTreeSet::from([index]), proposed.len(), samples, criteria)[index].members
                < criteria.minimum_members
        {
            outcomes[index].disposition = CategorySchemeDisposition::InsufficientMembers;
        }
        active.remove(&index);
    }
    Ok(outcomes)
}

fn record(candidate: &mut CategorySchemeCandidate, support: Support) {
    candidate.unique_members = support.members;
    candidate.mean_marginal_gain = if support.members == 0 {
        0.0
    } else {
        (support.gain / support.members as f64) as f32
    };
}

fn weakest(indices: &[usize], support: &[Support]) -> Option<usize> {
    indices.iter().copied().min_by(|left, right| {
        support[*left]
            .gain
            .total_cmp(&support[*right].gain)
            .then_with(|| support[*left].members.cmp(&support[*right].members))
            .then_with(|| right.cmp(left))
    })
}

fn support(
    active: &BTreeSet<usize>,
    count: usize,
    samples: &[ScoredSample],
    criteria: CategoryAdmissionCriteria,
) -> Vec<Support> {
    let mut support = vec![Support::default(); count];
    for sample in samples {
        if let Some((index, separation)) = supporting_candidate(active, sample, criteria) {
            support[index].members += 1;
            support[index].gain += f64::from(separation);
        }
    }
    support
}

pub(in crate::application::classification_discovery) fn supporting_candidate(
    active: &BTreeSet<usize>,
    sample: &ScoredSample,
    criteria: CategoryAdmissionCriteria,
) -> Option<(usize, f32)> {
    let mut best = sample.baseline.similarity;
    let mut second = best;
    let mut leading = None;
    for index in active {
        let score = sample.proposed[*index];
        if score > best {
            second = best;
            best = score;
            leading = Some(*index);
        } else {
            second = second.max(score);
        }
    }
    let index = leading?;
    let improvement = best - sample.baseline.similarity;
    let separation = best - second;
    if sample.eligible[index]
        && improvement > 0.0
        && improvement >= criteria.minimum_improvement
        && separation > 0.0
        && separation >= criteria.minimum_separation
    {
        Some((index, separation))
    } else {
        None
    }
}
