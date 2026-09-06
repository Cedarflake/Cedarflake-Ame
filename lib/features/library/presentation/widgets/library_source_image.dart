import "dart:io";

import "package:flutter/painting.dart";

import "../../domain/library_models.dart";

/// Keeps Flutter's file decoder while separating cache entries by source lease.
class LibrarySourceImage extends FileImage {
  LibrarySourceImage(LibraryAsset asset, {this.retryGeneration = 0})
    : sourceVersion = (
        rootId: asset.rootId,
        locationId: asset.locationId,
        fileSize: asset.fileSize,
        modifiedUnixMs: asset.modifiedUnixMs,
        identityScheme: asset.fileIdentity?.scheme,
        identityValue: asset.fileIdentity?.value,
        revision: asset.sourceRevision,
        generation: asset.sourceGeneration,
      ),
      super(File(asset.sourcePath));

  final ({
    String rootId,
    String locationId,
    BigInt fileSize,
    int modifiedUnixMs,
    String? identityScheme,
    String? identityValue,
    LibrarySourceRevisionEvidence? revision,
    BigInt generation,
  })
  sourceVersion;
  final int retryGeneration;

  @override
  bool operator ==(Object other) =>
      other is LibrarySourceImage &&
      super == other &&
      sourceVersion == other.sourceVersion &&
      retryGeneration == other.retryGeneration;

  @override
  int get hashCode =>
      Object.hash(super.hashCode, sourceVersion, retryGeneration);
}
