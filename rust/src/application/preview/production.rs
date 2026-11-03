use crate::ports::SourceMediaInspector;
use crate::ports::preview_source::PreviewSourceFactory;

pub(super) fn sources() -> impl PreviewSourceFactory {
    crate::adapters::LocalPreviewSources
}

pub(super) fn media_inspector() -> impl SourceMediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
