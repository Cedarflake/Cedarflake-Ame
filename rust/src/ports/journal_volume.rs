use std::path::Path;
use std::sync::Arc;

use crate::domain::{
    LibraryRootGeneration, PersistentJournalBaseline, PersistentJournalReadFailure, ScanError,
};

use super::{PersistentJournalNamespaceReader, PersistentJournalVolumeReader};

pub(crate) trait PersistentJournalVolumeSource {
    /// The caller retains these root handles through capture and catalog publication.
    type Registration;
    type Reader: PersistentJournalVolumeReader;

    fn open_root(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        root_path: &Path,
    ) -> Result<Self::Registration, PersistentJournalReadFailure>;

    fn into_reader(
        self,
        registrations: &[Self::Registration],
        namespace: Arc<dyn PersistentJournalNamespaceReader>,
        baselines: Vec<PersistentJournalBaseline>,
    ) -> Result<Self::Reader, ScanError>;
}
