use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::{
    CategoryAdmissionCriteria, CategoryAdmissionPolicy, CategoryDefinition, CategoryId,
    CategoryMatcher, ClassificationContent, ClassificationEncoderFactory, ClassificationRecordId,
    ClassificationRevision, DiscoverySample, Embedding, ImageFeature, ModelCompatibility, Taxonomy,
    TextFeature, assess_category_scheme,
};
use rust_lib_cedarflake_ame::classification_worker::LocalClassificationEncoderFactory;
use serde::Deserialize;
use serde_json::json;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Fixture {
    categories: Vec<Category>,
    baseline_ids: Vec<String>,
    samples: Vec<Sample>,
    minimum_members: usize,
    budgets: Vec<usize>,
}

#[derive(Deserialize)]
struct Category {
    id: String,
    label: String,
    descriptions: Vec<Description>,
}

#[derive(Deserialize)]
struct Description {
    text: String,
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct Sample {
    id: String,
    values: Vec<f32>,
}

pub(super) fn run(path: &OsStr) -> Result<()> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Scheme fixture exceeds its byte window".into());
    }
    let fixture: Fixture = serde_json::from_slice(&bytes)?;
    if fixture.categories.len() > 256 || fixture.samples.len() > 512 || fixture.budgets.len() > 8 {
        return Err("Scheme fixture exceeds its work window".into());
    }
    let model = LocalClassificationEncoderFactory {
        executable: Default::default(),
        model_directory: Default::default(),
        runtime_directory: Default::default(),
    }
    .compatibility();
    let expanded = matcher(fixture.categories.iter(), model)?;
    let baseline = matcher(
        fixture
            .categories
            .iter()
            .filter(|category| fixture.baseline_ids.contains(&category.id)),
        model,
    )?;
    let identities: BTreeMap<_, _> = fixture
        .samples
        .iter()
        .map(|sample| (*blake3::hash(sample.id.as_bytes()).as_bytes(), &sample.id))
        .collect();
    let mut samples = fixture
        .samples
        .iter()
        .map(|sample| {
            Ok(DiscoverySample {
                content: ClassificationContent {
                    scheme: ClassificationRecordId::new("cached-calibration".into())?,
                    digest: *blake3::hash(sample.id.as_bytes()).as_bytes(),
                    byte_length: 1,
                },
                feature: ImageFeature {
                    identity: model.image_identity(),
                    embedding: Embedding::normalized(sample.values.clone())?,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let labels: BTreeMap<_, _> = fixture
        .categories
        .iter()
        .map(|category| (category.id.as_str(), category.label.as_str()))
        .collect();
    for budget in fixture.budgets {
        let policy = CategoryAdmissionPolicy::new(
            ClassificationRevision::from_bytes(*blake3::hash(&bytes).as_bytes()),
            CategoryAdmissionCriteria {
                minimum_members: fixture.minimum_members,
                coverage_per_thousand: 500,
                minimum_improvement: 0.0,
                minimum_separation: 0.0,
                minimum_context_contrast: 0.0,
                max_proposals: budget,
            },
        )?;
        let started = Instant::now();
        let result = assess_category_scheme(
            &baseline,
            &expanded,
            &samples,
            &policy,
            &AtomicBool::new(false),
        )?;
        samples.reverse();
        let repeat = assess_category_scheme(
            &baseline,
            &expanded,
            &samples,
            &policy,
            &AtomicBool::new(false),
        )?;
        if result != repeat {
            return Err("Reordered category scheme samples changed the assessment".into());
        }
        println!(
            "{}",
            json!({
                "budget": budget, "samples": samples.len(), "seconds": started.elapsed().as_secs_f64(),
                "repeat_equal": true, "source_reads": 0, "new_inference": 0,
                "revision": result.revision.bytes(),
                "model": {"space": model.image_identity().embedding_space.bytes(), "image": model.image_identity().pipeline.bytes(), "text": model.text_identity().pipeline.bytes()},
                "candidates": result.candidates.iter().map(|candidate| json!({
                    "id": candidate.category_id.as_str(), "label": labels[candidate.category_id.as_str()],
                    "disposition": format!("{:?}", candidate.disposition),
                    "unique_members": candidate.unique_members, "mean_marginal_gain": candidate.mean_marginal_gain,
                })).collect::<Vec<_>>(),
                "members": result.members.iter().map(|member| json!({
                    "id": identities[&member.content.digest], "previous": member.previous_category.as_str(),
                    "proposed": member.proposed_category.as_str(),
                })).collect::<Vec<_>>(),
            })
        );
    }
    Ok(())
}

fn matcher<'a>(
    categories: impl Iterator<Item = &'a Category>,
    model: ModelCompatibility,
) -> Result<CategoryMatcher> {
    let mut definitions = Vec::new();
    let mut features = BTreeMap::new();
    for category in categories {
        definitions.push(CategoryDefinition::new(
            CategoryId::new(category.id.clone())?,
            category.label.clone(),
            category
                .descriptions
                .iter()
                .map(|description| description.text.clone())
                .collect(),
        )?);
        for description in &category.descriptions {
            let revision = ClassificationRevision::description(&description.text);
            let feature = TextFeature {
                identity: model.text_identity(),
                description: revision,
                embedding: Embedding::normalized(description.values.clone())?,
            };
            if features
                .insert(revision, feature.clone())
                .is_some_and(|existing| existing != feature)
            {
                return Err("Conflicting description features in the scheme fixture".into());
            }
        }
    }
    Ok(CategoryMatcher::new(
        Taxonomy::new(definitions)?,
        model,
        features.into_values().collect(),
    )?)
}
