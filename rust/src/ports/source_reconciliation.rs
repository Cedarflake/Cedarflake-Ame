use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, PreviewRequest, ScanError,
};

pub(crate) enum SourceReconciliationScope<'a> {
    Exact(&'a str),
    Relocated { previous: &'a str, current: &'a str },
}

impl<'a> SourceReconciliationScope<'a> {
    pub(crate) fn from_intent(intent: &'a LibraryChangeIntent) -> Option<Self> {
        if intent.origin != LibraryChangeOrigin::ConsistencyAudit
            || intent.scope != LibraryChangeScope::Path
            || intent.relative_path.is_empty()
        {
            return None;
        }
        match (intent.kind, intent.previous_relative_path.as_deref()) {
            (LibraryChangeIntentKind::Reconcile, None) => Some(Self::Exact(&intent.relative_path)),
            (LibraryChangeIntentKind::RenameCandidate, Some(previous))
                if !previous.is_empty() && previous != intent.relative_path =>
            {
                Some(Self::Relocated {
                    previous,
                    current: &intent.relative_path,
                })
            }
            _ => None,
        }
    }

    pub(crate) fn previous_path(&self) -> &str {
        match self {
            Self::Exact(path) => path,
            Self::Relocated { previous, .. } => previous,
        }
    }

    pub(crate) fn admits_paths(&self, paths: &[String]) -> bool {
        match self {
            Self::Exact(path) => matches!(paths, [actual] if actual.as_str() == *path),
            Self::Relocated { previous, current } => {
                paths.len() == 2
                    && paths.iter().any(|path| path.as_str() == *previous)
                    && paths.iter().any(|path| path.as_str() == *current)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum SourceReconciliationAdmission {
    Leased(Box<LeasedLibraryChange>),
    RequestSuperseded,
    ExistingPathWork,
    LeaseUnavailable,
}

pub(crate) trait SourceReconciliationRepository {
    fn admit_source_reconciliation(
        &mut self,
        request: &PreviewRequest,
        intent: &LibraryChangeIntent,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<SourceReconciliationAdmission, ScanError>;
}
