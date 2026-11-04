use crate::ports::SourceMediaInspector;
use crate::ports::preview_source::PreviewSourceFactory;
use crate::ports::source_identity::SourceIdentityLocator;

pub(super) fn sources() -> impl PreviewSourceFactory + SourceIdentityLocator {
    crate::adapters::LocalPreviewSources
}

pub(super) fn media_inspector() -> impl SourceMediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
