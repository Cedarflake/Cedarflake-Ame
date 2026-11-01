use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::{
    DiscoveryCohortBatch, DiscoveryCohortPolicy, form_discovery_cohorts,
};
use serde_json::{Value, json};

use super::{Result, baseline, identities, load_fixture, model, samples};

pub(crate) fn run(path: &OsStr) -> Result<()> {
    let (fixture, revision) = load_fixture(path)?;
    let model = model();
    let matcher = baseline(&fixture, model)?;
    let identities = identities(&fixture);
    let mut samples = samples(&fixture, model)?;
    let policy = DiscoveryCohortPolicy {
        minimum_pair_cosine: 0.75,
        minimum_members: 3,
        maximum_cohorts: 64,
    };
    let started = Instant::now();
    let result = form_discovery_cohorts(&samples, &matcher, policy, &AtomicBool::new(false))?;
    let first_seconds = started.elapsed().as_secs_f64();
    let encoded = encode(&result, &identities);
    samples.reverse();
    let started = Instant::now();
    let repeat = form_discovery_cohorts(&samples, &matcher, policy, &AtomicBool::new(false))?;
    let repeat_seconds = started.elapsed().as_secs_f64();
    if encoded != encode(&repeat, &identities) {
        return Err("Reordered cached features changed the discovery cohorts".into());
    }
    println!(
        "{}",
        json!({
            "fixture_revision": revision.bytes(), "samples": samples.len(),
            "first_seconds": first_seconds, "repeat_seconds": repeat_seconds, "repeat_equal": true,
            "source_reads": 0, "new_inference": 0,
            "policy": {"minimum_pair_cosine": policy.minimum_pair_cosine,
                "minimum_members": policy.minimum_members, "maximum_cohorts": policy.maximum_cohorts},
            "result": encoded,
        })
    );
    Ok(())
}

fn encode(result: &DiscoveryCohortBatch, identities: &BTreeMap<[u8; 32], &str>) -> Value {
    json!({
        "cohorts": result.cohorts.iter().map(|cohort| json!({
            "revision": cohort.revision().bytes(),
            "members": cohort.member_contents().map(|content| identities[&content.digest]).collect::<Vec<_>>(),
            "comparison": cohort.comparison_contents().iter().map(|content| identities[&content.digest]).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "deferred": result.deferred.iter().map(|cohort| json!({
            "reason": format!("{:?}", cohort.reason),
            "members": cohort.members.iter().map(|content| identities[&content.digest]).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}
