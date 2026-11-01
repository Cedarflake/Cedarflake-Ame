use super::{AtomicBool, DiscoverySample, ScanError, check_cancelled, invalid};

pub(super) fn groups(
    samples: &[&DiscoverySample],
    minimum_similarity: f32,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<usize>>, ScanError> {
    let count = samples.len();
    let mut links = vec![f32::NEG_INFINITY; count * count];
    for left in 0..count {
        check_cancelled(cancelled)?;
        for right in left + 1..count {
            links[left * count + right] = samples[left]
                .feature
                .embedding
                .cosine(&samples[right].feature.embedding)
                .map_err(|_| invalid())?;
            links[right * count + left] = links[left * count + right];
        }
    }
    let mut clusters: Vec<_> = (0..count).map(|index| vec![index]).collect();
    loop {
        check_cancelled(cancelled)?;
        let mut closest = None;
        let mut best = minimum_similarity;
        for left in 0..count {
            check_cancelled(cancelled)?;
            if clusters[left].is_empty() {
                continue;
            }
            for right in left + 1..count {
                if clusters[right].is_empty() {
                    continue;
                }
                let score = links[left * count + right];
                if score >= minimum_similarity && (closest.is_none() || score > best) {
                    closest = Some((left, right));
                    best = score;
                }
            }
        }
        let Some((left, right)) = closest else { break };
        for other in 0..count {
            if other == left || other == right || clusters[other].is_empty() {
                continue;
            }
            let complete_link = links[left * count + other].min(links[right * count + other]);
            links[left * count + other] = complete_link;
            links[other * count + left] = complete_link;
        }
        let joined = std::mem::take(&mut clusters[right]);
        clusters[left].extend(joined);
        clusters[left].sort_unstable();
    }
    Ok(clusters
        .into_iter()
        .filter(|cluster| !cluster.is_empty())
        .collect())
}
