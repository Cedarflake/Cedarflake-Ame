use std::collections::BTreeMap;
use std::sync::{OnceLock, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, ConceptNamingScope};
use crate::domain::classification::maintenance::managed_category_identity;
use crate::domain::classification::{CategoryDefinition, CategoryId};
use crate::ports::classification_vocabulary::ManagedClassificationVocabulary;

use super::super::{classification_candidate, classification_runtime};
use super::invalid;

pub(super) fn categories() -> Result<&'static BTreeMap<CategoryId, CategoryDefinition>, ScanError> {
    static DEFAULTS: OnceLock<Result<BTreeMap<CategoryId, CategoryDefinition>, ScanError>> =
        OnceLock::new();
    DEFAULTS.get_or_init(load).as_ref().map_err(Clone::clone)
}

fn load() -> Result<BTreeMap<CategoryId, CategoryDefinition>, ScanError> {
    let baseline = classification_candidate::initial_categories().map_err(invalid)?;
    let mut definitions: BTreeMap<_, _> = baseline
        .categories()
        .iter()
        .map(|category| (category.id().clone(), category.clone()))
        .collect();
    let vocabulary = classification_runtime::vocabulary(&AtomicBool::new(false))?;
    let contexts = vocabulary.naming_contexts();
    for naming in classification_runtime::bundled_naming()? {
        let CategoryNaming::WithinCategory(purpose) = &naming else {
            continue;
        };
        for context in &contexts {
            let concept = naming.project(&context.concept);
            if concept.naming_scope() != ConceptNamingScope::Candidate {
                continue;
            }
            let id = managed_category_identity(concept.id(), Some(purpose.category()))
                .map_err(invalid)?;
            let definition = CategoryDefinition::new(
                id.clone(),
                concept.label().into(),
                vec![concept.description().into()],
            )
            .map_err(invalid)?;
            definitions.insert(id, definition);
        }
    }
    Ok(definitions)
}
