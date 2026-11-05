use super::{SIMILARITY_MAX_DISTANCE, SimilarVariant};

pub(crate) fn confirmed_distance(left: &SimilarVariant, right: &SimilarVariant) -> Option<u8> {
    let candidate = distance(&left.candidate, &right.candidate);
    if candidate > SIMILARITY_MAX_DISTANCE {
        return None;
    }
    let confirmation: u16 = left
        .confirmation
        .iter()
        .zip(&right.confirmation)
        .map(|(left, right)| (left ^ right).count_ones() as u16)
        .sum();
    (confirmation <= 44).then_some(candidate)
}

pub(crate) fn distance(left: &[u16; 4], right: &[u16; 4]) -> u8 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones() as u8)
        .sum()
}

pub(crate) fn band_neighbors(word: u16) -> Vec<u16> {
    let mut result = Vec::with_capacity(137);
    result.push(word);
    for first in 0..16 {
        result.push(word ^ (1 << first));
        for second in first + 1..16 {
            result.push(word ^ (1 << first) ^ (1 << second));
        }
    }
    result.sort_unstable();
    result
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MatchPhase {
    References,
    Peer,
    Members,
    Complete,
}

impl MatchPhase {
    pub(crate) fn exhausted(self, found_reference: bool) -> Self {
        match self {
            Self::References if !found_reference => Self::Peer,
            _ => Self::Complete,
        }
    }
}
