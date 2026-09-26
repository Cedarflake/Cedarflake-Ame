import "../domain/library_models.dart";
import "library_previewer.dart";

bool isLibraryPreviewRootFailure(LibraryPreviewFailure failure) =>
    switch (failure.code) {
      "preview_root_unavailable" ||
      "preview_root_identity_unproven" ||
      "preview_root_identity_changed" ||
      "preview_root_not_found" => true,
      _ => false,
    };

LibraryAsset? failedLibraryPreviewReplacement(
  LibraryAsset asset,
  Object error,
) {
  // A failed materialization command does not revoke an existing artifact.
  // Confirmed corruption arrives as a failed asset from the application owner.
  if (asset.previewStatus == LibraryPreviewStatus.ready &&
      asset.previewPath.isNotEmpty) {
    return null;
  }
  return asset.withPreview(
    previewPath: asset.previewPath,
    width: asset.width,
    height: asset.height,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: error is LibraryPreviewFailure
        ? error.code
        : "preview_request_failed",
    previewIssueMessage: error is LibraryPreviewFailure
        ? error.message
        : error.toString(),
  );
}
