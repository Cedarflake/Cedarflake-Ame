use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use crate::adapters::{
    LocalPreviewStore, PreviewCacheNamespace, open_preview_cache_operation_namespace,
};
use crate::application::preview_cleanup::{PreviewGenerationGuard, PreviewReclamationGuard};
use crate::application::{StoragePaths, acquire_preview_generation, acquire_preview_reclamation};
use crate::domain::ScanError;

use super::PreviewStageTimings;

static ACTIVE_STORE: OnceLock<Mutex<Option<ActiveStore>>> = OnceLock::new();

struct ActiveStore {
    root: std::path::PathBuf,
    budget_bytes: u64,
    store: Arc<LocalPreviewStore>,
}

#[derive(Clone, Copy)]
pub(in crate::application) enum PreviewStoreSource<'a> {
    Active(&'a StoragePaths),
    #[cfg(test)]
    Injected {
        storage: &'a StoragePaths,
        store: &'a LocalPreviewStore,
    },
}

enum ResolvedStore<'a> {
    Active {
        storage: &'a StoragePaths,
        store: Arc<LocalPreviewStore>,
    },
    #[cfg(test)]
    Injected {
        storage: &'a StoragePaths,
        store: &'a LocalPreviewStore,
    },
}

pub(in crate::application) struct PreviewStoreAdmission<'a, Access> {
    store: ResolvedStore<'a>,
    _access: Access,
    namespace: Arc<PreviewCacheNamespace>,
}

pub(super) struct PreviewOperationSource<'a> {
    source: PreviewStoreSource<'a>,
    namespace: Arc<PreviewCacheNamespace>,
}

pub(in crate::application) type PreviewGenerationAdmission<'a> =
    PreviewStoreAdmission<'a, PreviewGenerationGuard>;
pub(in crate::application) type PreviewReclamationAdmission<'a> =
    PreviewStoreAdmission<'a, PreviewReclamationGuard>;

impl<'a> PreviewStoreSource<'a> {
    pub(super) fn generation(
        self,
        timings: &mut PreviewStageTimings,
    ) -> Result<PreviewGenerationAdmission<'a>, ScanError> {
        let access = acquire_generation_access(timings)?;
        let started = Instant::now();
        let operation = self.operation()?;
        timings.store_ms = timings
            .store_ms
            .saturating_add(started.elapsed().as_millis());
        operation.generation_with_access(access, timings)
    }

    #[cfg(test)]
    pub(in crate::application) fn reclamation(
        self,
    ) -> Result<PreviewReclamationAdmission<'a>, ScanError> {
        let access = acquire_preview_reclamation()?;
        self.operation()?.reclamation_with_access(access)
    }

    fn operation(self) -> Result<PreviewOperationSource<'a>, ScanError> {
        let storage = match self {
            Self::Active(storage) => storage,
            #[cfg(test)]
            Self::Injected { storage, .. } => storage,
        };
        Ok(PreviewOperationSource {
            source: self,
            namespace: Arc::new(open_preview_cache_operation_namespace(
                &storage.preview_root,
            )?),
        })
    }

    fn resolve(self, namespace: &PreviewCacheNamespace) -> Result<ResolvedStore<'a>, ScanError> {
        match self {
            Self::Active(storage) => Ok(ResolvedStore::Active {
                storage,
                store: resolve_active_store(storage, namespace)?,
            }),
            #[cfg(test)]
            Self::Injected { storage, store } => {
                if !namespace.admits(&storage.preview_root)
                    || store.namespace_identity() != namespace.identity()
                {
                    return Err(ScanError::new(
                        "preview_cache_namespace_mismatch",
                        "The injected store belongs to a different directory namespace",
                    ));
                }
                Ok(ResolvedStore::Injected { storage, store })
            }
        }
    }
}

impl<'a> PreviewOperationSource<'a> {
    pub(super) fn generation(
        &self,
        timings: &mut PreviewStageTimings,
    ) -> Result<PreviewGenerationAdmission<'a>, ScanError> {
        self.generation_with_access(acquire_generation_access(timings)?, timings)
    }

    fn generation_with_access(
        &self,
        access: PreviewGenerationGuard,
        timings: &mut PreviewStageTimings,
    ) -> Result<PreviewGenerationAdmission<'a>, ScanError> {
        let started = Instant::now();
        let store = self.source.resolve(&self.namespace)?;
        timings.store_ms = timings
            .store_ms
            .saturating_add(started.elapsed().as_millis());
        Ok(PreviewStoreAdmission {
            store,
            _access: access,
            namespace: Arc::clone(&self.namespace),
        })
    }

    pub(in crate::application) fn reclamation(
        &self,
    ) -> Result<PreviewReclamationAdmission<'a>, ScanError> {
        self.reclamation_with_access(acquire_preview_reclamation()?)
    }

    fn reclamation_with_access(
        &self,
        access: PreviewReclamationGuard,
    ) -> Result<PreviewReclamationAdmission<'a>, ScanError> {
        let store = self.source.resolve(&self.namespace)?;
        Ok(PreviewStoreAdmission {
            store,
            _access: access,
            namespace: Arc::clone(&self.namespace),
        })
    }
}

impl<'a, Access> PreviewStoreAdmission<'a, Access> {
    pub(in crate::application) fn storage(&self) -> &'a StoragePaths {
        match &self.store {
            ResolvedStore::Active { storage, .. } => storage,
            #[cfg(test)]
            ResolvedStore::Injected { storage, .. } => storage,
        }
    }

    pub(in crate::application) fn store(&self) -> &LocalPreviewStore {
        match &self.store {
            ResolvedStore::Active { store, .. } => store,
            #[cfg(test)]
            ResolvedStore::Injected { store, .. } => store,
        }
    }

    pub(super) fn source(&self) -> PreviewOperationSource<'a> {
        let source = match &self.store {
            ResolvedStore::Active { storage, .. } => PreviewStoreSource::Active(storage),
            #[cfg(test)]
            ResolvedStore::Injected { storage, store } => {
                PreviewStoreSource::Injected { storage, store }
            }
        };
        PreviewOperationSource {
            source,
            namespace: Arc::clone(&self.namespace),
        }
    }
}

// Only the admission constructors resolve a production store: access must be held first,
// and releasing access retires this selection even if another request still retains its Arc.
fn resolve_active_store(
    storage: &StoragePaths,
    namespace: &PreviewCacheNamespace,
) -> Result<Arc<LocalPreviewStore>, ScanError> {
    {
        let active = slot().lock().map_err(|_| registry_unavailable())?;
        if let Some(store) = matching_store(&active, storage, namespace) {
            return Ok(store);
        }
    }
    let candidate = Arc::new(
        LocalPreviewStore::new_in_namespace(
            storage.preview_root.clone(),
            storage.preview_budget_bytes,
            namespace,
        )
        .map_err(|issue| {
            ScanError::new(
                issue.code,
                format!("Preview cache initialization failed: {}", issue.message),
            )
        })?,
    );
    // Inventory does not hold the registry mutex. Concurrent generation admissions converge
    // on the already-published owner; exclusive invalidation cannot cross their access guard.
    let mut active = slot().lock().map_err(|_| registry_unavailable())?;
    if let Some(store) = matching_store(&active, storage, namespace) {
        return Ok(store);
    }
    *active = Some(ActiveStore {
        root: storage.preview_root.clone(),
        budget_bytes: storage.preview_budget_bytes,
        store: Arc::clone(&candidate),
    });
    Ok(candidate)
}

fn matching_store(
    active: &Option<ActiveStore>,
    storage: &StoragePaths,
    namespace: &PreviewCacheNamespace,
) -> Option<Arc<LocalPreviewStore>> {
    active
        .as_ref()
        .filter(|active| {
            active.root == storage.preview_root
                && active.budget_bytes == storage.preview_budget_bytes
                && active.store.namespace_identity() == namespace.identity()
        })
        .map(|active| Arc::clone(&active.store))
}

pub(crate) fn invalidate_active_preview_store() -> Result<(), ScanError> {
    *slot().lock().map_err(|_| registry_unavailable())? = None;
    Ok(())
}

#[cfg(test)]
pub(crate) fn active_preview_store(
    storage: &StoragePaths,
) -> Result<Arc<LocalPreviewStore>, ScanError> {
    let namespace = PreviewCacheNamespace::for_operation(&storage.preview_root)?;
    resolve_active_store(storage, &namespace)
}

fn slot() -> &'static Mutex<Option<ActiveStore>> {
    ACTIVE_STORE.get_or_init(Mutex::default)
}

fn acquire_generation_access(
    timings: &mut PreviewStageTimings,
) -> Result<PreviewGenerationGuard, ScanError> {
    let started = Instant::now();
    let access = acquire_preview_generation()?;
    timings.access_ms = timings
        .access_ms
        .saturating_add(started.elapsed().as_millis());
    Ok(access)
}

fn registry_unavailable() -> ScanError {
    ScanError::new(
        "preview_store_registry_unavailable",
        "Preview store registry is poisoned",
    )
}
