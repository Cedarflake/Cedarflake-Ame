use std::path::Path;

use crate::adapters::{SqliteCatalog, is_current_preview_artifact, user_visible_path};
use crate::domain::{
    AssetLocationView, DiscoveredFile, MediaInspection, MetadataInspection, PreviewStatus,
    ScanError,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, MediaInspectionFailure, MediaInspector,
};

use super::{stable_id, stable_location_id};

pub(super) struct PreparedScanFile<'a> {
    scan_id: &'a str,
    root_id: &'a str,
    file: DiscoveredFile,
    asset_id: String,
    location_id: String,
    preservation_prior: Option<AssetLocationView>,
    reusable_prior: Option<AssetLocationView>,
    can_reuse_metadata: bool,
}

impl<'a> PreparedScanFile<'a> {
    pub fn load(
        catalog: &SqliteCatalog,
        inspector: &impl MediaInspector,
        scan_id: &'a str,
        root_id: &'a str,
        file: DiscoveredFile,
        has_active_locations: bool,
    ) -> Result<Self, ScanError> {
        let path_prior = if has_active_locations {
            catalog.load_incremental_location_by_relative_path(root_id, &file.relative_path)?
        } else {
            None
        };
        let location_id = path_prior.as_ref().map_or_else(
            || stable_location_id(root_id, &file.relative_path),
            |prior| prior.location_id.clone(),
        );
        let candidate_asset_id = file.file_identity.as_ref().map_or_else(
            || stable_id("asset-v1", &format!("{scan_id}\0{location_id}")),
            |identity| {
                stable_id(
                    "asset-file-identity-v1",
                    &format!("{scan_id}\0{}\0{}", identity.scheme, identity.value),
                )
            },
        );
        let identity_prior = match file.file_identity.as_ref() {
            Some(identity)
                if path_prior
                    .as_ref()
                    .and_then(|prior| prior.file_identity.as_ref())
                    == Some(identity) =>
            {
                path_prior.clone()
            }
            Some(identity) => catalog.load_scan_location_by_file_identity(scan_id, identity)?,
            None => None,
        };
        let path_is_unchanged = path_prior.as_ref().is_some_and(|prior| {
            same_file_state(prior, &file)
                && (file.file_identity.is_none()
                    || prior.file_identity.is_none()
                    || prior.file_identity == file.file_identity)
        });
        let asset_id = if let Some(prior) = &identity_prior {
            prior.asset_id.clone()
        } else if let Some(prior) = path_prior.as_ref().filter(|_| path_is_unchanged) {
            prior.asset_id.clone()
        } else {
            candidate_asset_id
        };
        let preservation_prior = identity_prior.clone().or_else(|| path_prior.clone());
        let reusable_prior = identity_prior
            .filter(|prior| same_file_state(prior, &file))
            .or_else(|| path_is_unchanged.then_some(path_prior).flatten());
        let can_reuse_metadata = reusable_prior.as_ref().is_some_and(|prior| {
            prior.metadata_engine_id == inspector.metadata_engine_id()
                && prior.metadata_engine_version == inspector.metadata_engine_version()
        });
        Ok(Self {
            scan_id,
            root_id,
            file,
            asset_id,
            location_id,
            preservation_prior,
            reusable_prior,
            can_reuse_metadata,
        })
    }

    pub fn file(&self) -> &DiscoveredFile {
        &self.file
    }

    pub fn preservation_prior(&self) -> Option<&AssetLocationView> {
        self.preservation_prior.as_ref()
    }

    pub fn inspect(
        &self,
        inspector: &impl MediaInspector,
    ) -> Result<MediaInspection, MediaInspectionFailure> {
        if let Some(prior) = self
            .reusable_prior
            .as_ref()
            .filter(|_| self.can_reuse_metadata)
        {
            return Ok(MediaInspection {
                width: prior.width,
                height: prior.height,
                metadata: MetadataInspection {
                    engine_id: prior.metadata_engine_id.clone(),
                    engine_version: prior.metadata_engine_version.clone(),
                    capture_time: prior.capture_time.clone(),
                    issues: Vec::new(),
                },
            });
        }
        inspector.inspect(&self.file)
    }

    pub fn into_asset(self, inspection: MediaInspection) -> AssetLocationView {
        let (preview_path, preview_status) = self
            .reusable_prior
            .as_ref()
            .filter(|prior| {
                self.can_reuse_metadata
                    && matches!(prior.preview_status, PreviewStatus::Ready)
                    && !prior.preview_path.is_empty()
                    && Path::new(&prior.preview_path).is_file()
                    && is_current_preview_artifact(&prior.preview_path)
            })
            .map(|prior| (prior.preview_path.clone(), PreviewStatus::Ready))
            .unwrap_or_else(|| (String::new(), PreviewStatus::Pending));
        let source_generation = self
            .reusable_prior
            .as_ref()
            .filter(|prior| same_file_state(prior, &self.file))
            .map_or(0, |prior| prior.source_generation);
        AssetLocationView {
            asset_id: self.asset_id,
            location_id: self.location_id,
            root_id: self.root_id.to_owned(),
            scan_id: self.scan_id.to_owned(),
            display_path: user_visible_path(&self.file.absolute_path),
            absolute_path: self.file.absolute_path,
            relative_path: self.file.relative_path,
            preview_path,
            file_size: self.file.file_size,
            created_unix_ms: self.file.created_unix_ms,
            modified_unix_ms: self.file.modified_unix_ms,
            file_identity: self.file.file_identity,
            source_revision: self.file.source_revision,
            source_generation,
            width: inspection.width,
            height: inspection.height,
            preview_status,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: inspection.metadata.engine_id,
            metadata_engine_version: inspection.metadata.engine_version,
            capture_time: inspection.metadata.capture_time,
        }
    }
}

fn same_file_state(prior: &AssetLocationView, file: &DiscoveredFile) -> bool {
    prior.file_size == file.file_size
        && prior.modified_unix_ms == file.modified_unix_ms
        && prior.source_revision.is_some()
        && prior.source_revision == file.source_revision
}
