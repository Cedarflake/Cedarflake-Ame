use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adapters::{LocalClassificationVocabulary, SqliteCatalog};
use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;

use super::super::classification_discovery::{
    ContextualVocabulary, DiscoveryPreparationRequest, DiscoverySampleCursor, NamingContextCatalog,
    prepare_category_reduction, prepare_discovery_window,
};
use super::super::classification_dispatch::{
    ClassificationMaintenance, ClassificationMaintenanceStep,
};
use super::executor::NativeExecutor;

mod policy;
use policy::ManagedDiscoveryPolicy;

enum Pass {
    New,
    Sampling {
        notification: u64,
        cursor: Option<Box<DiscoverySampleCursor>>,
    },
    Reducing {
        notification: u64,
    },
    Complete {
        notification: u64,
    },
}

pub(super) struct NativeCategoryDiscovery {
    application_directory: PathBuf,
    pass: Pass,
    vocabulary: Option<(LocalClassificationVocabulary, NamingContextCatalog)>,
}

impl NativeCategoryDiscovery {
    pub(super) fn new(application_directory: PathBuf) -> Self {
        Self {
            application_directory,
            pass: Pass::New,
            vocabulary: None,
        }
    }
}

impl ClassificationMaintenance<SqliteCatalog, NativeExecutor> for NativeCategoryDiscovery {
    type Prepared = CategoryMaintenancePlan;

    fn prepare(
        &mut self,
        repository: &SqliteCatalog,
        executor: &mut NativeExecutor,
        notification: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationMaintenanceStep<CategoryMaintenancePlan>, ScanError> {
        match self.prepare_pass(repository, executor, notification, cancelled) {
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "classification_changed"
                        | "classification_samples_changed"
                        | "classification_discovery_changed"
                ) =>
            {
                self.pass = Pass::Complete { notification };
                self.vocabulary = None;
                Ok(ClassificationMaintenanceStep::Idle)
            }
            result => result,
        }
    }

    fn release_resources(&mut self) {
        self.vocabulary = None;
    }
}

impl NativeCategoryDiscovery {
    fn prepare_pass(
        &mut self,
        repository: &SqliteCatalog,
        executor: &mut NativeExecutor,
        notification: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationMaintenanceStep<CategoryMaintenancePlan>, ScanError> {
        match &self.pass {
            Pass::Complete {
                notification: previous,
            } if *previous == notification => {
                self.vocabulary = None;
                return Ok(ClassificationMaintenanceStep::Idle);
            }
            Pass::Sampling {
                notification: previous,
                ..
            }
            | Pass::Reducing {
                notification: previous,
            } if *previous == notification => {}
            _ => {
                self.pass = Pass::Sampling {
                    notification,
                    cursor: None,
                }
            }
        }
        let policy = ManagedDiscoveryPolicy::for_taxonomy(executor.taxonomy())?;
        if policy.is_available() && self.vocabulary.is_none() {
            let (source, contexts) = LocalClassificationVocabulary::open_bundled(
                &self.application_directory,
                cancelled,
            )?;
            self.vocabulary = Some((source, NamingContextCatalog::new(contexts)?));
        }
        if matches!(self.pass, Pass::Reducing { .. }) {
            let plan = prepare_category_reduction(
                operation_id()?,
                repository,
                self.vocabulary.as_ref().map(|(_, contexts)| contexts),
                cancelled,
            )?;
            self.pass = Pass::Complete { notification };
            self.vocabulary = None;
            return Ok(match plan {
                Some(plan) => ClassificationMaintenanceStep::Prepared(plan),
                None => ClassificationMaintenanceStep::Idle,
            });
        }
        if !policy.is_available() {
            self.pass = Pass::Reducing { notification };
            return Ok(ClassificationMaintenanceStep::Progress);
        }
        let cursor = match &self.pass {
            Pass::Sampling { cursor, .. } => cursor.as_deref(),
            _ => unreachable!("the discovery pass was admitted"),
        };
        let operation_id = operation_id()?;
        let resources = executor.feature_resources()?;
        let (source, contexts) = self.vocabulary.take().expect("the vocabulary was loaded");
        let mut vocabulary = ContextualVocabulary::new(source, contexts, cancelled);
        let result = prepare_discovery_window(
            DiscoveryPreparationRequest {
                operation_id,
                cursor,
                policy: &policy.preparation(),
            },
            repository,
            &mut vocabulary,
            resources.residency,
            resources.cache,
            cancelled,
        );
        self.vocabulary = Some(vocabulary.into_parts());
        let prepared = result?;
        self.pass = match prepared.next {
            Some(cursor) => Pass::Sampling {
                notification,
                cursor: Some(Box::new(cursor)),
            },
            None => Pass::Reducing { notification },
        };
        if let Some(plan) = prepared.expansion {
            return Ok(ClassificationMaintenanceStep::Prepared(
                CategoryMaintenancePlan::Expansion(Box::new(plan)),
            ));
        }
        Ok(ClassificationMaintenanceStep::Progress)
    }
}

fn operation_id() -> Result<ClassificationRecordId, ScanError> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ScanError::new("classification_clock_unavailable", error.to_string()))?
        .as_millis();
    ClassificationRecordId::new(format!(
        "discovery-{}-{millis}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed),
    ))
    .map_err(|error| ScanError::new("classification_operation_invalid", error.to_string()))
}
