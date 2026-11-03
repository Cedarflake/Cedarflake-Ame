import "../../../src/rust/domain.dart" as rust_domain;

import "../domain/library_models.dart";

LibraryAsset mapRustLibraryAsset(rust_domain.AssetLocationView asset) {
  final captureTime = asset.captureTime;
  final fileIdentity = asset.fileIdentity;
  final sourceRevision = asset.sourceRevision;
  return LibraryAsset(
    assetId: asset.assetId,
    locationId: asset.locationId,
    rootId: asset.rootId,
    activeScanId: asset.scanId,
    sourcePath: asset.absolutePath,
    displayPath: asset.displayPath,
    relativePath: asset.relativePath,
    previewPath: asset.previewPath,
    fileSize: asset.fileSize,
    createdUnixMs: asset.createdUnixMs,
    modifiedUnixMs: asset.modifiedUnixMs,
    sourceRevision: sourceRevision == null
        ? null
        : LibrarySourceRevisionEvidence(
            scheme: sourceRevision.scheme,
            value: sourceRevision.value,
          ),
    sourceGeneration: asset.sourceGeneration,
    width: asset.width,
    height: asset.height,
    previewStatus: switch (asset.previewStatus) {
      rust_domain.PreviewStatus.pending => LibraryPreviewStatus.pending,
      rust_domain.PreviewStatus.ready => LibraryPreviewStatus.ready,
      rust_domain.PreviewStatus.failed => LibraryPreviewStatus.failed,
    },
    previewIssueCode: asset.previewIssueCode,
    previewIssueMessage: asset.previewIssueMessage,
    metadataEngineId: asset.metadataEngineId,
    metadataEngineVersion: asset.metadataEngineVersion,
    camera: LibraryCameraMetadata(
      make: asset.camera.make,
      model: asset.camera.model,
    ),
    captureTime: captureTime == null
        ? null
        : LibraryCaptureTimeEvidence(
            localTime: captureTime.localTime,
            offsetMinutes: captureTime.offsetMinutes,
            source: switch (captureTime.source) {
              rust_domain.CaptureTimeSource.original =>
                LibraryCaptureTimeSource.exifDateTimeOriginal,
              rust_domain.CaptureTimeSource.digitized =>
                LibraryCaptureTimeSource.exifDateTimeDigitized,
              rust_domain.CaptureTimeSource.image =>
                LibraryCaptureTimeSource.exifDateTime,
            },
            rawValue: captureTime.rawValue,
          ),
    fileIdentity: fileIdentity == null
        ? null
        : LibraryFileIdentityEvidence(
            scheme: fileIdentity.scheme,
            value: fileIdentity.value,
          ),
  );
}
