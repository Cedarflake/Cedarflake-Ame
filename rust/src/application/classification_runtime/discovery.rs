use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::ports::classification_configuration::{
    CategoryExpansionRepository, CategoryMergeRepository, CategoryRetirementRepository,
};
use crate::ports::classification_vocabulary::ManagedClassificationVocabulary;

use super::super::classification_discovery::{
    DiscoveryPreparationRequest, DiscoverySampleCursor, NamingContextCatalog,
    prepare_category_catalog_reconciliation, prepare_category_reduction, prepare_discovery_window,
};
use super::super::classification_dispatch::{
    ClassificationMaintenance, ClassificationMaintenanceStep,
};
use super::executor::NativeExecutor;

mod policy;
use policy::ManagedDiscoveryPolicy;

enum Pass {
    New,
    Reconciling {
        notification: u64,
    },
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

pub(super) struct CategoryDiscovery<V> {
    pass: Pass,
    vocabulary: Option<(V, NamingContextCatalog)>,
    open_vocabulary: fn(&AtomicBool) -> Result<V, ScanError>,
}

impl<V: ManagedClassificationVocabulary> CategoryDiscovery<V> {
    pub(super) fn new(open_vocabulary: fn(&AtomicBool) -> Result<V, ScanError>) -> Self {
        Self {
            pass: Pass::New,
            vocabulary: None,
            open_vocabulary,
        }
    }
}

impl<R, V> ClassificationMaintenance<R, NativeExecutor> for CategoryDiscovery<V>
where
    R: CategoryExpansionRepository + CategoryMergeRepository + CategoryRetirementRepository,
    V: ManagedClassificationVocabulary,
{
    type Prepared = CategoryMaintenancePlan;

    fn prepare(
        &mut self,
        repository: &R,
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

impl<V: ManagedClassificationVocabulary> CategoryDiscovery<V> {
    fn prepare_pass(
        &mut self,
        repository: &(
             impl CategoryExpansionRepository + CategoryMergeRepository + CategoryRetirementRepository
         ),
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
            Pass::Reconciling {
                notification: previous,
            }
            | Pass::Sampling {
                notification: previous,
                ..
            }
            | Pass::Reducing {
                notification: previous,
            } if *previous == notification => {}
            _ => self.pass = Pass::Reconciling { notification },
        }
        let policy = ManagedDiscoveryPolicy::for_taxonomy(executor.taxonomy())?;
        if self.vocabulary.is_none() {
            let vocabulary = (self.open_vocabulary)(cancelled)?;
            let contexts = NamingContextCatalog::new(vocabulary.naming_contexts())?;
            self.vocabulary = Some((vocabulary, contexts));
        }
        if matches!(self.pass, Pass::Reconciling { .. }) {
            let vocabulary = &self
                .vocabulary
                .as_ref()
                .expect("the vocabulary was loaded")
                .0;
            let plan = prepare_category_catalog_reconciliation(
                operation_id()?,
                vocabulary.managed_catalog(),
                repository,
                cancelled,
            )?;
            self.pass = Pass::Sampling {
                notification,
                cursor: None,
            };
            return Ok(match plan {
                Some(plan) => ClassificationMaintenanceStep::Prepared(
                    CategoryMaintenancePlan::CatalogReconciliation(Box::new(plan)),
                ),
                None => ClassificationMaintenanceStep::Progress,
            });
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
        let (vocabulary, _) = self.vocabulary.as_mut().expect("the vocabulary was loaded");
        let prepared = prepare_discovery_window(
            DiscoveryPreparationRequest {
                operation_id,
                cursor,
                policy: &policy.preparation(),
            },
            repository,
            vocabulary,
            resources.residency,
            resources.cache,
            cancelled,
        )?;
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
