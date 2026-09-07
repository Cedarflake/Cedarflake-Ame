use std::path::Path;

use crate::domain::ScanError;

use super::{PreviewCacheNamespace, cache_inventory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreviewRootPreparation {
    Ready,
    NamespaceUnavailable,
}

pub(crate) fn open_preview_cache_operation_namespace(
    root: &Path,
) -> Result<PreviewCacheNamespace, ScanError> {
    PreviewCacheNamespace::for_operation(root).map_err(|error| {
        ScanError::new(
            "preview_cache_namespace_unavailable",
            format!(
                "无法安全写入当前预览缓存。图库仍可浏览；请在设置中选择可访问的本地 NTFS 缓存目录并重启。尚未生成或删除预览文件。{}",
                error.message
            ),
        )
    })
}

pub(crate) fn prepare_preview_root(root: &Path) -> Result<PreviewRootPreparation, ScanError> {
    prepare_preview_root_with_namespace_probe(root, open_preview_cache_operation_namespace)
}

pub(crate) fn prepare_preview_root_with_namespace_probe(
    root: &Path,
    probe: impl FnOnce(&Path) -> Result<PreviewCacheNamespace, ScanError>,
) -> Result<PreviewRootPreparation, ScanError> {
    let namespace = match probe(root) {
        Ok(namespace) => namespace,
        // Capability refusal is isolated from catalog startup, not converted into write authority.
        Err(_) => return Ok(PreviewRootPreparation::NamespaceUnavailable),
    };
    if !namespace.admits(root) {
        return Err(ScanError::new(
            "preview_cache_namespace_mismatch",
            "Preview preparation does not own the requested directory namespace",
        ));
    }
    cache_inventory(root)
        .map(|_| PreviewRootPreparation::Ready)
        .map_err(|issue| ScanError::new(issue.code, issue.message))
}

#[cfg(test)]
mod tests;
