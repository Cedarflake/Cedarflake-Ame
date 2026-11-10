use std::path::{Path, PathBuf};

use super::{
    AssetLocationView, IncrementalCatalogRoot, MediaInspection, ScanIssue, user_visible_path,
};

#[derive(Clone, Debug)]
pub struct FileOperationDestinationPublication {
    pub root: IncrementalCatalogRoot,
    pub location: AssetLocationView,
}

pub struct FileOperationOutputMetadata {
    pub created_unix_ms: Option<i64>,
    pub inspection: MediaInspection,
    pub issue: Option<ScanIssue>,
}

pub(crate) fn relative_output_path(root: &str, absolute: &str) -> Option<String> {
    let root_path = user_visible_path(root);
    let output_path = user_visible_path(absolute);
    let expected = Path::new(&root_path).components().collect::<Vec<_>>();
    let actual = Path::new(&output_path).components().collect::<Vec<_>>();
    if expected.len() >= actual.len()
        || !expected.iter().zip(&actual).all(|(left, right)| {
            left.as_os_str().to_string_lossy().to_lowercase()
                == right.as_os_str().to_string_lossy().to_lowercase()
        })
    {
        return None;
    }
    let relative = actual[expected.len()..]
        .iter()
        .map(|component| component.as_os_str())
        .collect::<PathBuf>();
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}
