use crate::application::materialize_preview;
use crate::domain::{AssetLocationView, PreviewRequest, ScanError};

pub fn materialize_library_preview(
    request: PreviewRequest,
) -> Result<AssetLocationView, ScanError> {
    materialize_preview(request)
}
