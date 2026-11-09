use crate::domain::IncrementalCatalogRoot;
use crate::domain::file_operation_publication::FileOperationOutputMetadata;
use crate::domain::file_operations::{FileOperationIssue, FileOperationSource};

pub trait FileOperationOutput {
    fn metadata(&self) -> Option<&FileOperationOutputMetadata>;
    fn revalidate(&self) -> Result<(), FileOperationIssue>;
}

pub trait FileOperationOutputReader {
    type Output: FileOperationOutput;

    fn hold_output(
        &self,
        source: &FileOperationSource,
        root: Option<(&IncrementalCatalogRoot, &str)>,
    ) -> Result<Self::Output, FileOperationIssue>;
}
