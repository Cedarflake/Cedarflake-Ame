use crate::domain::{FileIdentityEvidence, LibraryRootGeneration, ScanError};

pub(crate) trait SourceIdentityLocator {
    /// Resolves metadata within the expected root without hydrating or modifying source files.
    /// Unsupported identity lookup returns None; discovered paths still require publication checks.
    fn current_paths(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        root_path: &str,
        expected_root_identity: &FileIdentityEvidence,
        file_identity: &FileIdentityEvidence,
    ) -> Result<Option<Vec<String>>, ScanError>;
}
