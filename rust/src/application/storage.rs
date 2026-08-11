use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use directories::ProjectDirs;

use crate::adapters::{
    LocalPreviewStore, PREVIEW_CACHE_VERSION, SqliteCatalog, SqliteStorageSettings,
    is_ame_preview_cache_entry, user_visible_path,
};
use crate::domain::{
    GalleryQuery, RetiredPreviewRootView, ScanError, StorageConfiguration, StorageSettingsUpdate,
    StorageStatus,
};
use crate::ports::{CatalogRepository, StorageSettingsRepository};

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
    Ok(storage)
}

pub fn load_storage_status() -> Result<StorageStatus, ScanError> {
    let active = storage_paths()?;
    let configured = load_configured_storage(&active)?;
    storage_status(&active, &configured)
}

pub fn update_storage_settings(update: StorageSettingsUpdate) -> Result<StorageStatus, ScanError> {
    validate_budget(update.preview_budget_bytes)?;
    let active = storage_paths()?;
    let mut configured = load_configured_storage(&active)?;

    if let Some(catalog_directory) = update.catalog_directory {
        let directory = validate_absolute_directory(&catalog_directory, "catalog")?;
        let candidate = directory.join("ame.sqlite3");
        if !resolved_paths_same(&candidate, &active.catalog_path)?
            && active_catalog_has_roots(&active.catalog_path)?
        {
            return Err(ScanError::new(
                "catalog_relocation_requires_migration",
                "The catalog already contains library roots and cannot be relocated without an explicit migration",
            ));
        }
        configured.catalog_path = candidate.to_string_lossy().into_owned();
    }
    if let Some(preview_cache_directory) = update.preview_cache_directory {
        let directory = validate_absolute_directory(&preview_cache_directory, "preview cache")?;
        configured.preview_root = directory
            .join(PREVIEW_CACHE_VERSION)
            .to_string_lossy()
            .into_owned();
    }
    configured.preview_budget_bytes = update.preview_budget_bytes;
    validate_configuration_paths(&active.catalog_path, &configured)?;

    let configured_catalog = Path::new(&configured.catalog_path);
    if let Some(parent) = configured_catalog.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ScanError::new(
                "configured_catalog_directory_unavailable",
                format!("Could not prepare the configured catalog directory: {error}"),
            )
        })?;
    }
    fs::create_dir_all(&configured.preview_root).map_err(|error| {
        ScanError::new(
            "configured_preview_directory_unavailable",
            format!("Could not prepare the configured preview directory: {error}"),
        )
    })?;
    let mut settings = SqliteStorageSettings::open(active.settings_path.clone())?;
    let active_preview_root = active.preview_root.to_string_lossy();
    let pending_preview_root =
        (!resolved_paths_same(Path::new(&configured.preview_root), &active.preview_root)?)
            .then_some(active_preview_root.as_ref());
    settings.save(&configured, pending_preview_root)?;
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

fn activate_configured_preview_root(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
) -> Result<PathBuf, ScanError> {
    activate_configured_preview_root_with(
        settings_path,
        catalog_path,
        configured,
        initialize_preview_root,
    )
}

fn activate_configured_preview_root_with(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
    mut initialize: impl FnMut(&Path, u64) -> Result<(), ScanError>,
) -> Result<PathBuf, ScanError> {
    let target = PathBuf::from(&configured.preview_root);
    let mut settings = SqliteStorageSettings::open(settings_path.to_path_buf())?;
    let pending_roots = settings.load_pending_preview_roots()?;
    if let Err(target_error) = initialize(&target, configured.preview_budget_bytes) {
        for pending_root in pending_roots {
            let previous = PathBuf::from(pending_root);
            if initialize(&previous, configured.preview_budget_bytes).is_ok() {
                return Ok(previous);
            }
        }
        return Err(target_error);
    }

    if !pending_roots.is_empty() && catalog_path.exists() {
        let mut catalog = SqliteCatalog::open(catalog_path.to_path_buf())?;
        catalog.reset_previews_outside_root(&preview_root_prefix(&target))?;
    }
    settings.activate_preview_root(&configured.preview_root)?;
    Ok(target)
}

fn initialize_preview_root(path: &Path, budget_bytes: u64) -> Result<(), ScanError> {
    LocalPreviewStore::new(path.to_path_buf(), budget_bytes)
        .map(|_| ())
        .map_err(|issue| ScanError::new(issue.code, issue.message))
}

fn preview_root_prefix(path: &Path) -> String {
    let mut prefix = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned();
    prefix.push(std::path::MAIN_SEPARATOR);
    prefix
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

fn validate_configuration_paths(
    active_catalog_path: &Path,
    configuration: &StorageConfiguration,
) -> Result<(), ScanError> {
    if !active_catalog_path.exists() {
        return Ok(());
    }
    let mut catalog = SqliteCatalog::open(active_catalog_path.to_path_buf())?;
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
    for (label, storage_path) in [
        ("catalog", storage.catalog_path.as_path()),
        ("preview cache", storage.preview_root.as_path()),
        ("settings", storage.settings_path.as_path()),
    ] {
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
    let mut catalog = SqliteCatalog::open(catalog_path.to_path_buf())?;
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
    let mut catalog = SqliteCatalog::open(path.to_path_buf())?;
    let query = GalleryQuery::default();
    Ok(!catalog
        .load_snapshot(1, &query, "storage-validation", None, None, None)?
        .roots
        .is_empty())
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = normalized_path(left);
    let right = normalized_path(right);
    left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\"))
}

pub(crate) fn resolved_paths_overlap(left: &Path, right: &Path) -> Result<bool, ScanError> {
    if paths_overlap(left, right) {
        return Ok(true);
    }
    let left = resolved_normalized_path(left)?;
    let right = resolved_normalized_path(right)?;
    Ok(left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\")))
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
        catalog_used_bytes: catalog_size(&active.catalog_path)?,
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
mod tests {
    use super::*;
    use crate::ports::StorageSettingsRepository;
    use tempfile::tempdir;

    #[test]
    fn overlap_detection_is_case_insensitive_and_bidirectional() {
        assert!(paths_overlap(
            Path::new("E:\\ExampleLibrary"),
            Path::new("e:\\examplelibrary\\.ame-cache"),
        ));
        assert!(paths_overlap(
            Path::new("E:\\ExampleLibrary\\nested"),
            Path::new("E:\\ExampleLibrary"),
        ));
        assert!(!paths_overlap(
            Path::new("E:\\ExampleLibrary"),
            Path::new("E:\\AmeCache"),
        ));
    }

    #[test]
    fn resolved_comparison_collapses_existing_path_aliases() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("source");
        let nested = source.join("nested");
        let previews = source.join("previews");
        fs::create_dir_all(&nested).expect("nested source");
        fs::create_dir_all(&previews).expect("previews");
        let aliased_previews = nested.join("..").join("previews");

        assert!(resolved_paths_same(&aliased_previews, &previews).expect("resolved comparison"));
        assert!(resolved_paths_overlap(&source, &aliased_previews).expect("resolved overlap"));
    }

    #[test]
    fn resolved_comparison_normalizes_nonexistent_parent_segments() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("source");
        let previews = source.join("previews");
        fs::create_dir_all(&source).expect("source");
        let disguised_previews = directory
            .path()
            .join("not-created")
            .join("..")
            .join("source")
            .join("previews");

        assert!(
            resolved_paths_same(&disguised_previews, &previews)
                .expect("normalized resolved comparison")
        );
        assert!(
            resolved_paths_overlap(&source, &disguised_previews)
                .expect("normalized resolved overlap")
        );
    }

    #[test]
    fn source_root_cannot_contain_active_storage() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("source");
        let preview_root = source.join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };

        let error = validate_source_root_storage_paths(&source, &storage)
            .expect_err("overlapping source must be rejected");

        assert_eq!(error.code, "source_root_overlaps_storage");
    }

    #[test]
    fn preview_budget_has_explicit_supported_bounds() {
        assert!(validate_budget(MIN_PREVIEW_BUDGET_BYTES).is_ok());
        assert!(validate_budget(MAX_PREVIEW_BUDGET_BYTES).is_ok());
        assert_eq!(
            validate_budget(MIN_PREVIEW_BUDGET_BYTES - 1)
                .expect_err("small budget")
                .code,
            "preview_budget_invalid"
        );
    }

    #[test]
    fn saved_configuration_is_pending_until_the_process_restarts() {
        let storage = tempdir().expect("storage");
        let active = StoragePaths {
            catalog_path: storage.path().join("active").join("ame.sqlite3"),
            preview_root: storage.path().join("active-previews"),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let initial = load_configured_storage(&active).expect("initial settings");
        assert_eq!(initial.catalog_path, active.catalog_path.to_string_lossy());

        let configured = StorageConfiguration {
            catalog_path: initial.catalog_path,
            preview_root: storage
                .path()
                .join("next-previews")
                .to_string_lossy()
                .into_owned(),
            preview_budget_bytes: 8 * 1024 * 1024 * 1024,
        };
        SqliteStorageSettings::open(active.settings_path.clone())
            .expect("settings")
            .save(&configured, Some(&active.preview_root.to_string_lossy()))
            .expect("save configuration");

        let status = storage_status(&active, &configured).expect("status");

        assert!(status.requires_restart);
        assert_eq!(
            status.active_preview_root,
            active.preview_root.to_string_lossy()
        );
        assert_eq!(status.configured_preview_root, configured.preview_root);
    }

    #[test]
    fn successful_preview_root_activation_retires_the_previous_root() {
        let storage = tempdir().expect("storage");
        let settings_path = storage.path().join("settings.sqlite3");
        let old_root = storage.path().join("old-previews");
        let target_root = storage.path().join("target-previews");
        let defaults = StorageConfiguration {
            catalog_path: storage
                .path()
                .join("catalog.sqlite3")
                .to_string_lossy()
                .into_owned(),
            preview_root: old_root.to_string_lossy().into_owned(),
            preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
        };
        let configured = StorageConfiguration {
            preview_root: target_root.to_string_lossy().into_owned(),
            ..defaults.clone()
        };
        let mut settings = SqliteStorageSettings::open(settings_path.clone()).expect("settings");
        settings
            .load_or_initialize(&defaults)
            .expect("initialize settings");
        settings
            .save(&configured, Some(&defaults.preview_root))
            .expect("save pending target");
        drop(settings);

        let active_root = activate_configured_preview_root_with(
            &settings_path,
            Path::new(&configured.catalog_path),
            &configured,
            |_, _| Ok(()),
        )
        .expect("activate target");

        assert_eq!(active_root, target_root);
        let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
        assert!(
            settings
                .load_pending_preview_roots()
                .expect("pending roots")
                .is_empty()
        );
        assert_eq!(
            settings
                .load_retired_preview_roots()
                .expect("retired roots"),
            vec![defaults.preview_root]
        );
    }

    #[test]
    fn failed_preview_root_activation_keeps_the_previous_root_pending() {
        let storage = tempdir().expect("storage");
        let settings_path = storage.path().join("settings.sqlite3");
        let old_root = storage.path().join("old-previews");
        let target_root = storage.path().join("target-previews");
        let defaults = StorageConfiguration {
            catalog_path: storage
                .path()
                .join("catalog.sqlite3")
                .to_string_lossy()
                .into_owned(),
            preview_root: old_root.to_string_lossy().into_owned(),
            preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
        };
        let configured = StorageConfiguration {
            preview_root: target_root.to_string_lossy().into_owned(),
            ..defaults.clone()
        };
        let mut settings = SqliteStorageSettings::open(settings_path.clone()).expect("settings");
        settings
            .load_or_initialize(&defaults)
            .expect("initialize settings");
        settings
            .save(&configured, Some(&defaults.preview_root))
            .expect("save pending target");
        drop(settings);

        let active_root = activate_configured_preview_root_with(
            &settings_path,
            Path::new(&configured.catalog_path),
            &configured,
            |path, _| {
                if path == target_root {
                    Err(ScanError::new(
                        "preview_cache_unavailable",
                        "target unavailable",
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .expect("fall back to previous root");

        assert_eq!(active_root, old_root);
        let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
        assert_eq!(
            settings
                .load_pending_preview_roots()
                .expect("pending roots"),
            vec![defaults.preview_root]
        );
        assert!(
            settings
                .load_retired_preview_roots()
                .expect("retired roots")
                .is_empty()
        );
    }

    #[test]
    fn storage_status_separates_access_and_display_paths() {
        let storage = tempdir().expect("storage");
        let active = StoragePaths {
            catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let configured = StorageConfiguration {
            catalog_path: r"\\?\C:\AmeData\ame.sqlite3".to_owned(),
            preview_root: r"\\?\C:\AmeCache\previews".to_owned(),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        };

        let status = storage_status(&active, &configured).expect("status");

        assert_eq!(status.configured_catalog_path, configured.catalog_path);
        assert_eq!(
            status.configured_catalog_display_path,
            r"C:\AmeData\ame.sqlite3",
        );
        assert_eq!(
            status.configured_preview_display_path,
            r"C:\AmeCache\previews",
        );
    }

    #[test]
    fn storage_status_counts_current_and_legacy_preview_bytes() {
        let storage = tempdir().expect("storage");
        let preview_root = storage.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let current = preview_root.join(format!("{PREVIEW_CACHE_VERSION}-{}.jpg", "a".repeat(64)));
        let legacy = preview_root.join(format!("{}.jpg", "b".repeat(64)));
        let foreign = preview_root.join("keep.jpg");
        fs::write(current, vec![1_u8; 7]).expect("current preview");
        fs::write(legacy, vec![2_u8; 13]).expect("legacy preview");
        fs::write(foreign, vec![3_u8; 17]).expect("foreign file");
        let active = StoragePaths {
            catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
            preview_root: preview_root.clone(),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let configured = StorageConfiguration {
            catalog_path: active.catalog_path.to_string_lossy().into_owned(),
            preview_root: preview_root.to_string_lossy().into_owned(),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        };

        let status = storage_status(&active, &configured).expect("storage status");

        assert_eq!(status.preview_used_bytes, 20);
    }
}
