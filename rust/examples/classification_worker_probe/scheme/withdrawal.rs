use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::assess_category_withdrawals;
use serde_json::json;

use super::{Result, identities, load_fixture, matcher, model, samples};

pub(crate) fn run(path: &OsStr) -> Result<()> {
    let (fixture, revision) = load_fixture(path)?;
    let model = model();
    let matcher = matcher(fixture.categories.iter(), model)?;
    let identities = identities(&fixture);
    let labels: BTreeMap<_, _> = fixture
        .categories
        .iter()
        .map(|category| (category.id.as_str(), category.label.as_str()))
        .collect();
    let mut samples = samples(&fixture, model)?;
    let started = Instant::now();
    let result = assess_category_withdrawals(&matcher, &samples, &AtomicBool::new(false))?;
    let first_seconds = started.elapsed().as_secs_f64();
    samples.reverse();
    let repeat = assess_category_withdrawals(&matcher, &samples, &AtomicBool::new(false))?;
    if result != repeat {
        return Err("Reordered cached features changed category withdrawal impacts".into());
    }
    println!(
        "{}",
        json!({
            "fixture_revision": revision.bytes(), "assessment_revision": result.revision.bytes(),
            "samples": samples.len(), "first_seconds": first_seconds, "repeat_equal": true,
            "source_reads": 0, "new_inference": 0,
            "categories": result.categories.iter().map(|impact| json!({
                "id": impact.category_id.as_str(), "label": labels[impact.category_id.as_str()],
                "baseline": fixture.baseline_ids.iter().any(|id| id == impact.category_id.as_str()),
                "affected": impact.affected.iter().map(|member| json!({
                    "id": identities[&member.content.digest], "loss": member.similarity_loss(),
                    "removed_similarity": member.removed_similarity,
                    "alternative": member.alternative.category_id.as_str(),
                    "alternative_similarity": member.alternative.similarity,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    );
    Ok(())
}
