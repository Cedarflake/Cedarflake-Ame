class LibraryAsset {
  const LibraryAsset({
    required this.assetId,
    required this.locationId,
    required this.rootId,
    required this.activeScanId,
    required this.sourcePath,
    required this.displayPath,
    required this.relativePath,
    required this.previewPath,
    required this.fileSize,
    required this.modifiedUnixMs,
    required this.sourceRevision,
    required this.sourceGeneration,
    required this.width,
    required this.height,
    this.previewStatus = LibraryPreviewStatus.ready,
    this.previewIssueCode,
    this.previewIssueMessage,
    this.metadataEngineId = "unknown",
    this.metadataEngineVersion = "0",
    this.createdUnixMs,
    this.captureTime,
    this.camera = const LibraryCameraMetadata(),
    this.fileIdentity,
  });

  final String assetId;
  final String locationId;
  final String rootId;
  final String activeScanId;
  final String sourcePath;
  final String displayPath;
  final String relativePath;
  final String previewPath;
  final BigInt fileSize;
  final int? createdUnixMs;
  final int modifiedUnixMs;
  final LibrarySourceRevisionEvidence? sourceRevision;
  final BigInt sourceGeneration;
  final int width;
  final int height;
  final LibraryPreviewStatus previewStatus;
  final String? previewIssueCode;
  final String? previewIssueMessage;
  final String metadataEngineId;
  final String metadataEngineVersion;
  final LibraryCaptureTimeEvidence? captureTime;
  final LibraryCameraMetadata camera;
  final LibraryFileIdentityEvidence? fileIdentity;

  LibraryAsset withPreview({
    required String previewPath,
    required int width,
    required int height,
    required LibraryPreviewStatus previewStatus,
    String? previewIssueCode,
    String? previewIssueMessage,
  }) {
    return LibraryAsset(
      assetId: assetId,
      locationId: locationId,
      rootId: rootId,
      activeScanId: activeScanId,
      sourcePath: sourcePath,
      displayPath: displayPath,
      relativePath: relativePath,
      previewPath: previewPath,
      fileSize: fileSize,
      createdUnixMs: createdUnixMs,
      modifiedUnixMs: modifiedUnixMs,
      sourceRevision: sourceRevision,
      sourceGeneration: sourceGeneration,
      width: width,
      height: height,
      previewStatus: previewStatus,
      previewIssueCode: previewIssueCode,
      previewIssueMessage: previewIssueMessage,
      metadataEngineId: metadataEngineId,
      metadataEngineVersion: metadataEngineVersion,
      captureTime: captureTime,
      camera: camera,
      fileIdentity: fileIdentity,
    );
  }
}

class LibraryFileIdentityEvidence {
  const LibraryFileIdentityEvidence({
    required this.scheme,
    required this.value,
  });

  final String scheme;
  final String value;
}

class LibrarySourceRevisionEvidence {
  const LibrarySourceRevisionEvidence({
    required this.scheme,
    required this.value,
  });

  final String scheme;
  final String value;

  @override
  bool operator ==(Object other) {
    return other is LibrarySourceRevisionEvidence &&
        other.scheme == scheme &&
        other.value == value;
  }

  @override
  int get hashCode => Object.hash(scheme, value);
}

enum LibraryPreviewStatus { pending, ready, failed }

class LibraryCaptureTimeEvidence {
  const LibraryCaptureTimeEvidence({
    required this.localTime,
    required this.source,
    required this.rawValue,
    this.offsetMinutes,
  });

  final String localTime;
  final int? offsetMinutes;
  final LibraryCaptureTimeSource source;
  final String rawValue;
}

enum LibraryCaptureTimeSource {
  exifDateTimeOriginal,
  exifDateTimeDigitized,
  exifDateTime,
}

class LibraryCameraMetadata {
  const LibraryCameraMetadata({this.make, this.model});

  final String? make;
  final String? model;
}
