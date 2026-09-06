use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use directories::ProjectDirs;

#[cfg(test)]
use crate::adapters::SqliteCatalog;
use crate::adapters::{
    PREVIEW_CACHE_VERSION, SqliteStorageSettings, is_ame_preview_cache_entry, user_visible_path,
};
use crate::domain::{
    GalleryQuery, LibraryChangeLane, RetiredPreviewRootView, ScanError, StorageConfiguration,
    StorageSettingsUpdate, StorageStatus,
};
use crate::ports::{CatalogRepository, StorageSettingsRepository};

pub(super) mod catalog_admission;
mod configuration_update;
mod preview_activation;
use preview_activation::activate_configured_preview_root;
#[cfg(test)]
use preview_activation::activate_configured_preview_root_with;

const DEFAULT_PREVIEW_BUDGET_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MIN_PREVIEW_BUDGET_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PREVIEW_BUDGET_BYTES: u64 = 1024 * 1024 * 1024 * 1024;

static ACTIVE_STORAGE: OnceLock<Result<StoragePaths, ScanError>> = OnceLock::new();

#[derive(Clone, Debug)]
pub(crate) struct StoragePaths {
    pub(crate) catalog_path: PathBuf,
    pub(crate) preview_root: PathBuf,
    pub(crate) preview_budget_bytes: u64,
    pub(crate) settings_path: PathBuf,
}

pub(crate) fn storage_paths() -> Result<StoragePaths, ScanError> {
    #[cfg(debug_assertions)]
    if let Some(test_root) = std::env::var_os("CEDARFLAKE_AME_TEST_STORAGE_ROOT") {
        let test_root = PathBuf::from(test_root);
        if !test_root.is_absolute() {
            return Err(ScanError::new(
                "test_storage_path_invalid",
                "CEDARFLAKE_AME_TEST_STORAGE_ROOT must be an absolute path",
            ));
        }
        let storage = StoragePaths {
            catalog_path: test_root.join("catalog").join("ame.sqlite3"),
            preview_root: test_root
                .join("cache")
                .join("previews")
                .join(PREVIEW_CACHE_VERSION),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: test_root.join("settings").join("storage.sqlite3"),
        };
        return Ok(storage);
    }

    let storage = ACTIVE_STORAGE
        .get_or_init(resolve_configured_storage)
        .clone()?;
    #[cfg(not(test))]
    if storage.catalog_path.is_file() {
        super::schedule_catalog_reclamation_recovery(storage.catalog_path.clone());
    }
    Ok(storage)
}

pub fn load_storage_status() -> Result<StorageStatus, ScanError> {
    let active = storage_paths()?;
    let configured = load_configured_storage(&active)?;
    storage_status(&active, &configured)
}

pub fn update_storage_settings(update: StorageSettingsUpdate) -> Result<StorageStatus, ScanError> {
    let active = storage_paths()?;
    let configured = configuration_update::save_configuration(&active, update)?;
    storage_status(&active, &configured)
}

fn resolve_configured_storage() -> Result<StoragePaths, ScanError> {
    let (settings_path, defaults) = default_storage()?;
    let configured = load_or_initialize_configuration(&settings_path, &defaults)?;
    validate_configuration(&configured)?;
    let preview_root = activate_configured_preview_root(
        &settings_path,
        Path::new(&configured.catalog_path),
        &configured,
    )?;
    Ok(StoragePaths {
        catalog_path: PathBuf::from(configured.catalog_path),
        preview_root,
        preview_budget_bytes: configured.preview_budget_bytes,
        settings_path,
    })
}

fn load_configured_storage(active: &StoragePaths) -> Result<StorageConfiguration, ScanError> {
    let defaults = StorageConfiguration {
        catalog_path: active.catalog_path.to_string_lossy().into_owned(),
        preview_root: active.preview_root.to_string_lossy().into_owned(),
        preview_budget_bytes: active.preview_budget_bytes,
    };
    load_or_initialize_configuration(&active.settings_path, &defaults)
}

fn load_or_initialize_configuration(
    settings_path: &Path,
    defaults: &StorageConfiguration,
) -> Result<StorageConfiguration, ScanError> {
    let mut settings = SqliteStorageSettings::open(settings_path.to_path_buf())?;
    let configured = settings.load_or_initialize(defaults)?;
    validate_configuration(&configured)?;
    Ok(configured)
}

fn default_storage() -> Result<(PathBuf, StorageConfiguration), ScanError> {
    let Some(project_dirs) = ProjectDirs::from("com", "Cedarflake", "Ame") else {
        return Err(ScanError::new(
            "application_storage_unavailable",
            "The operating system did not provide application storage directories",
        ));
    };
    let settings_path = project_dirs
        .config_dir()
        .join("storage")
        .join("settings.sqlite3");
    let configuration = StorageConfiguration {
        catalog_path: project_dirs
            .data_local_dir()
            .join("catalog")
            .join("ame.sqlite3")
            .to_string_lossy()
            .into_owned(),
        preview_root: project_dirs
            .cache_dir()
            .join("previews")
            .join(PREVIEW_CACHE_VERSION)
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
    };
    Ok((settings_path, configuration))
}

fn validate_configuration(configuration: &StorageConfiguration) -> Result<(), ScanError> {
    if !Path::new(&configuration.catalog_path).is_absolute()
        || !Path::new(&configuration.preview_root).is_absolute()
    {
        return Err(ScanError::new(
            "storage_configuration_path_invalid",
            "Configured catalog and preview paths must be absolute",
        ));
    }
    validate_budget(configuration.preview_budget_bytes)
}

fn validate_budget(budget: u64) -> Result<(), ScanError> {
    if !(MIN_PREVIEW_BUDGET_BYTES..=MAX_PREVIEW_BUDGET_BYTES).contains(&budget) {
        return Err(ScanError::new(
            "preview_budget_invalid",
            "Preview budget must be between 64 MiB and 1 TiB",
        ));
    }
    Ok(())
}

fn validate_absolute_directory(value: &str, label: &str) -> Result<PathBuf, ScanError> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ScanError::new(
            "storage_directory_not_absolute",
            format!("The selected {label} directory must be absolute"),
        ));
    }
    Ok(path)
}

fn preserve_active_path_if_equivalent(
    candidate: PathBuf,
    active: &Path,
) -> Result<PathBuf, ScanError> {
    if resolved_paths_same(&candidate, active)? {
        return Ok(active.to_path_buf());
    }
    Ok(candidate)
}

fn validate_configuration_paths(
    active_catalog_path: &Path,
    configuration: &StorageConfiguration,
) -> Result<(), ScanError> {
    if !active_catalog_path.exists() {
        return Ok(());
    }
    let mut catalog =
        super::catalog_session::open_catalog(active_catalog_path, LibraryChangeLane::Recovery)?;
    let query = GalleryQuery::default();
    let snapshot = catalog.load_snapshot(1, &query, "storage-validation", None, None, None)?;
    let configured_catalog = Path::new(&configuration.catalog_path);
    let configured_preview = Path::new(&configuration.preview_root);
    for root in snapshot.roots {
        let source = Path::new(&root.path);
        if resolved_paths_overlap(source, configured_catalog)?
            || resolved_paths_overlap(source, configured_preview)?
        {
            return Err(ScanError::new(
                "storage_path_overlaps_source",
                format!("Configured storage overlaps the source root {}", root.path),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_source_root_storage_paths(
    source_root: &Path,
    storage: &StoragePaths,
) -> Result<(), ScanError> {
    validate_source_root_against_paths(
        source_root,
        [
            ("catalog", storage.catalog_path.as_path()),
            ("preview cache", storage.preview_root.as_path()),
            ("settings", storage.settings_path.as_path()),
        ],
    )?;
    if !storage.settings_path.exists() {
        return Ok(());
    }
    let defaults = StorageConfiguration {
        catalog_path: storage.catalog_path.to_string_lossy().into_owned(),
        preview_root: storage.preview_root.to_string_lossy().into_owned(),
        preview_budget_bytes: storage.preview_budget_bytes,
    };
    let mut settings = SqliteStorageSettings::open(storage.settings_path.clone())?;
    let configured = settings.load_or_initialize(&defaults)?;
    validate_source_root_against_paths(
        source_root,
        [
            ("configured catalog", Path::new(&configured.catalog_path)),
            (
                "configured preview cache",
                Path::new(&configured.preview_root),
            ),
        ],
    )
}

fn validate_source_root_against_paths<'a>(
    source_root: &Path,
    storage_paths: impl IntoIterator<Item = (&'a str, &'a Path)>,
) -> Result<(), ScanError> {
    for (label, storage_path) in storage_paths {
        if resolved_paths_overlap(source_root, storage_path)? {
            return Err(ScanError::new(
                "source_root_overlaps_storage",
                format!(
                    "The selected library root overlaps Ame {label} storage at {}",
                    storage_path.display()
                ),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_preview_root_outside_sources(
    catalog_path: &Path,
    preview_root: &Path,
) -> Result<(), ScanError> {
    if !catalog_path.exists() {
        return Ok(());
    }
    let mut catalog =
        super::catalog_session::open_catalog(catalog_path, LibraryChangeLane::Recovery)?;
    let snapshot = catalog.load_snapshot(
        1,
        &GalleryQuery::default(),
        "preview-cleanup-validation",
        None,
        None,
        None,
    )?;
    for root in snapshot.roots {
        if resolved_paths_overlap(Path::new(&root.path), preview_root)? {
            return Err(ScanError::new(
                "preview_root_overlaps_source",
                format!(
                    "The preview cache resolves inside the source root {}",
                    root.path
                ),
            ));
        }
    }
    Ok(())
}

fn active_catalog_has_roots(path: &Path) -> Result<bool, ScanError> {
    if !path.exists() {
        return Ok(false);
    }
    let mut catalog = super::catalog_session::open_catalog(path, LibraryChangeLane::Recovery)?;
    let query = GalleryQuery::default();
    Ok(!catalog
        .load_snapshot(1, &query, "storage-validation", None, None, None)?
        .roots
        .is_empty())
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = normalized_path(left);
    let right = normalized_path(right);
    normalized_path_is_within(&left, &right) || normalized_path_is_within(&right, &left)
}

pub(crate) fn resolved_paths_overlap(left: &Path, right: &Path) -> Result<bool, ScanError> {
    if paths_overlap(left, right) {
        return Ok(true);
    }
    let left = resolved_normalized_path(left)?;
    let right = resolved_normalized_path(right)?;
    Ok(normalized_path_is_within(&left, &right) || normalized_path_is_within(&right, &left))
}

pub(crate) fn resolved_path_is_within(path: &Path, root: &Path) -> Result<bool, ScanError> {
    let path_text = normalized_path(path);
    let root_text = normalized_path(root);
    if normalized_path_is_within(&path_text, &root_text) {
        return Ok(true);
    }
    let path_text = resolved_normalized_path(path)?;
    let root_text = resolved_normalized_path(root)?;
    Ok(normalized_path_is_within(&path_text, &root_text))
}

pub(crate) fn resolved_paths_same(left: &Path, right: &Path) -> Result<bool, ScanError> {
    if paths_same(left, right) {
        return Ok(true);
    }
    Ok(resolved_normalized_path(left)? == resolved_normalized_path(right)?)
}

pub(crate) fn paths_same(left: &Path, right: &Path) -> bool {
    normalized_path(left) == normalized_path(right)
}

fn normalized_path(path: &Path) -> String {
    normalize_path_text(&path.to_string_lossy())
}

fn resolved_normalized_path(path: &Path) -> Result<String, ScanError> {
    let mut existing = path;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            ScanError::new(
                "storage_path_resolution_failed",
                format!("Could not resolve the storage path {}", path.display()),
            )
        })?;
    }
    let resolved = fs::canonicalize(existing).map_err(|error| {
        ScanError::new(
            "storage_path_resolution_failed",
            format!(
                "Could not resolve the storage path {}: {error}",
                path.display()
            ),
        )
    })?;
    let suffix = path.strip_prefix(existing).map_err(|_| {
        ScanError::new(
            "storage_path_resolution_failed",
            format!("Could not compare the storage path {}", path.display()),
        )
    })?;
    let resolved = normalize_path_components(&resolved.join(suffix));
    Ok(normalize_path_text(&resolved.to_string_lossy()))
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn normalize_path_text(path: &str) -> String {
    let path = path.replace('/', "\\");
    let path = path
        .strip_prefix("\\\\?\\UNC\\")
        .map(|path| format!("\\\\{path}"))
        .unwrap_or_else(|| path.trim_start_matches("\\\\?\\").to_owned());
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn normalized_path_is_within(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}\\"))
}

fn storage_status(
    active: &StoragePaths,
    configured: &StorageConfiguration,
) -> Result<StorageStatus, ScanError> {
    let active_catalog_path = active.catalog_path.to_string_lossy().into_owned();
    let active_preview_root = active.preview_root.to_string_lossy().into_owned();
    let mut settings = SqliteStorageSettings::open(active.settings_path.clone())?;
    let retired_preview_roots = settings
        .load_retired_preview_roots()?
        .into_iter()
        .filter(|preview_root| {
            !paths_same(Path::new(preview_root), &active.preview_root)
                && !paths_same(Path::new(preview_root), Path::new(&configured.preview_root))
        })
        .map(|preview_root| RetiredPreviewRootView {
            display_path: user_visible_path(&preview_root),
            preview_root,
        })
        .collect();
    let catalog_used_bytes = catalog_size(&active.catalog_path)?;
    let catalog_reclamation = super::catalog_reclamation_snapshot(&active.catalog_path)?;
    let catalog_live_bytes = if catalog_reclamation.catalog_file_bytes == 0 {
        catalog_used_bytes
    } else {
        catalog_reclamation.live_bytes
    };
    Ok(StorageStatus {
        settings_path: active.settings_path.to_string_lossy().into_owned(),
        active_catalog_path: active_catalog_path.clone(),
        active_preview_root: active_preview_root.clone(),
        configured_catalog_path: configured.catalog_path.clone(),
        configured_preview_root: configured.preview_root.clone(),
        configured_catalog_display_path: user_visible_path(&configured.catalog_path),
        configured_preview_display_path: user_visible_path(&configured.preview_root),
        preview_budget_bytes: configured.preview_budget_bytes,
        preview_used_bytes: directory_size(&active.preview_root)?,
        catalog_used_bytes,
        catalog_live_bytes,
        catalog_reclaimable_bytes: catalog_reclamation.reclaimable_bytes,
        catalog_reclamation,
        requires_restart: !resolved_paths_same(
            Path::new(&active_catalog_path),
            Path::new(&configured.catalog_path),
        )? || !resolved_paths_same(
            Path::new(&active_preview_root),
            Path::new(&configured.preview_root),
        )? || active.preview_budget_bytes != configured.preview_budget_bytes,
        retired_preview_roots,
    })
}

fn directory_size(path: &Path) -> Result<u64, ScanError> {
    if !path.exists() {
        return Ok(0);
    }
    let entries = fs::read_dir(path).map_err(|error| {
        ScanError::new(
            "storage_usage_unavailable",
            format!("Could not inspect preview cache usage: {error}"),
        )
    })?;
    let mut size = 0_u64;
    for entry in entries {
        let entry = entry
            .map_err(|error| ScanError::new("storage_usage_unavailable", error.to_string()))?;
        if !is_ame_preview_cache_entry(&entry.path()) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(ScanError::new(
                    "storage_usage_unavailable",
                    error.to_string(),
                ));
            }
        };
        if metadata.is_file() {
            size = size.saturating_add(metadata.len());
        }
    }
    Ok(size)
}

fn catalog_size(path: &Path) -> Result<u64, ScanError> {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(0);
    };
    let Some(parent) = path.parent() else {
        return Ok(0);
    };
    let mut size = 0_u64;
    for suffix in ["", "-wal", "-shm"] {
        let candidate = parent.join(format!("{file_name}{suffix}"));
        if let Ok(metadata) = candidate.metadata() {
            size = size.saturating_add(metadata.len());
        }
    }
    Ok(size)
}

#[cfg(test)]
mod tests;
