class LibraryAsset {
  const LibraryAsset({
    required this.assetId,
    required this.locationId,
    required this.rootId,
    required this.sourcePath,
    required this.relativePath,
    required this.previewPath,
    required this.fileSize,
    required this.modifiedUnixMs,
    required this.width,
    required this.height,
    this.previewStatus = LibraryPreviewStatus.ready,
    this.previewIssueCode,
    this.previewIssueMessage,
    this.metadataEngineId = "unknown",
    this.metadataEngineVersion = "0",
    this.createdUnixMs,
    this.captureTime,
    this.fileIdentity,
  });

  final String assetId;
  final String locationId;
  final String rootId;
  final String sourcePath;
  final String relativePath;
  final String previewPath;
  final BigInt fileSize;
  final int? createdUnixMs;
  final int modifiedUnixMs;
  final int width;
  final int height;
  final LibraryPreviewStatus previewStatus;
  final String? previewIssueCode;
  final String? previewIssueMessage;
  final String metadataEngineId;
  final String metadataEngineVersion;
  final LibraryCaptureTimeEvidence? captureTime;
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
      sourcePath: sourcePath,
      relativePath: relativePath,
      previewPath: previewPath,
      fileSize: fileSize,
      createdUnixMs: createdUnixMs,
      modifiedUnixMs: modifiedUnixMs,
      width: width,
      height: height,
      previewStatus: previewStatus,
      previewIssueCode: previewIssueCode,
      previewIssueMessage: previewIssueMessage,
      metadataEngineId: metadataEngineId,
      metadataEngineVersion: metadataEngineVersion,
      captureTime: captureTime,
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

class LibraryRoot {
  const LibraryRoot({
    required this.id,
    required this.path,
    required this.createdUnixMs,
    required this.assetCount,
    required this.issueCount,
    this.activeScanId,
    this.availability = LibraryRootAvailability.unknown,
    this.availabilityMessage,
  });

  final String id;
  final String path;
  final String? activeScanId;
  final int createdUnixMs;
  final int assetCount;
  final int issueCount;
  final LibraryRootAvailability availability;
  final String? availabilityMessage;
}

enum LibraryRootAvailability {
  unknown,
  available,
  missing,
  inaccessible,
  offline,
}

class LibrarySnapshot {
  const LibrarySnapshot({
    required this.catalogPath,
    required this.revision,
    required this.queryId,
    required this.roots,
    required this.assets,
    this.previousCursor,
    this.nextCursor,
  });

  final String catalogPath;
  final BigInt revision;
  final String queryId;
  final List<LibraryRoot> roots;
  final List<LibraryAsset> assets;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
}

class LibraryCatalogCursor {
  const LibraryCatalogCursor({
    required this.revision,
    required this.queryId,
    required this.primaryMissing,
    required this.primaryText,
    required this.primaryNumber,
    required this.rootId,
    required this.locationId,
  });

  final BigInt revision;
  final String queryId;
  final bool primaryMissing;
  final String primaryText;
  final int primaryNumber;
  final String rootId;
  final String locationId;
}

enum LibraryGallerySortKey { captureTime, createdTime, modifiedTime, fileName }

enum LibraryGallerySortDirection { ascending, descending }

class LibraryGalleryQuery {
  const LibraryGalleryQuery({
    this.rootId,
    this.folderRelativePath,
    this.includeDescendants = true,
    this.searchText = "",
    this.sortKey = LibraryGallerySortKey.captureTime,
    this.sortDirection = LibraryGallerySortDirection.descending,
  });

  static const Object _unchanged = Object();

  final String? rootId;
  final String? folderRelativePath;
  final bool includeDescendants;
  final String searchText;
  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection sortDirection;

  bool get isChronological => sortKey != LibraryGallerySortKey.fileName;

  LibraryGalleryQuery copyWith({
    Object? rootId = _unchanged,
    Object? folderRelativePath = _unchanged,
    bool? includeDescendants,
    String? searchText,
    LibraryGallerySortKey? sortKey,
    LibraryGallerySortDirection? sortDirection,
  }) {
    return LibraryGalleryQuery(
      rootId: rootId == _unchanged ? this.rootId : rootId as String?,
      folderRelativePath: folderRelativePath == _unchanged
          ? this.folderRelativePath
          : folderRelativePath as String?,
      includeDescendants: includeDescendants ?? this.includeDescendants,
      searchText: searchText ?? this.searchText,
      sortKey: sortKey ?? this.sortKey,
      sortDirection: sortDirection ?? this.sortDirection,
    );
  }

  @override
  int get hashCode => Object.hash(
    rootId,
    folderRelativePath,
    includeDescendants,
    searchText,
    sortKey,
    sortDirection,
  );

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is LibraryGalleryQuery &&
            rootId == other.rootId &&
            folderRelativePath == other.folderRelativePath &&
            includeDescendants == other.includeDescendants &&
            searchText == other.searchText &&
            sortKey == other.sortKey &&
            sortDirection == other.sortDirection;
  }
}

class LibraryTimeBucket {
  const LibraryTimeBucket({
    required this.itemCount,
    required this.aspectRatioSum,
    this.monthKey,
  });

  final String? monthKey;
  final int itemCount;
  final double aspectRatioSum;

  bool get isUnknown => monthKey == null;
}

class LibraryTimeline {
  const LibraryTimeline({
    required this.revision,
    required this.queryId,
    required this.totalItems,
    required this.buckets,
  });

  final BigInt revision;
  final String queryId;
  final int totalItems;
  final List<LibraryTimeBucket> buckets;
}

class LibraryTimeAnchor {
  const LibraryTimeAnchor({
    required this.revision,
    required this.queryId,
    required this.itemOffset,
    this.monthKey,
  });

  final BigInt revision;
  final String queryId;
  final int itemOffset;
  final String? monthKey;
}

class LibraryIssue {
  const LibraryIssue({required this.code, required this.message, this.path});

  final String code;
  final String message;
  final String? path;
}

class RecoverableLibraryScan {
  const RecoverableLibraryScan({
    required this.scanId,
    required this.rootPath,
    required this.previewEdge,
    required this.visitedEntries,
    required this.acceptedItems,
    required this.issueCount,
    this.itemLimit,
    this.entryLimit,
  });

  final String scanId;
  final String rootPath;
  final int? itemLimit;
  final int? entryLimit;
  final int previewEdge;
  final int visitedEntries;
  final int acceptedItems;
  final int issueCount;
}

sealed class LibraryScanUpdate {
  const LibraryScanUpdate();
}

class LibraryScanStarted extends LibraryScanUpdate {
  const LibraryScanStarted({
    required this.scanId,
    required this.rootPath,
    this.itemLimit,
    this.entryLimit,
  });

  final String scanId;
  final String rootPath;
  final int? itemLimit;
  final int? entryLimit;
}

class LibraryScanProgress extends LibraryScanUpdate {
  const LibraryScanProgress({
    required this.visitedEntries,
    required this.acceptedItems,
    required this.issueCount,
  });

  final int visitedEntries;
  final int acceptedItems;
  final int issueCount;
}

class LibraryAssetDiscovered extends LibraryScanUpdate {
  const LibraryAssetDiscovered(this.asset);

  final LibraryAsset asset;
}

class LibraryIssueDiscovered extends LibraryScanUpdate {
  const LibraryIssueDiscovered(this.issue);

  final LibraryIssue issue;
}

class LibraryScanCompleted extends LibraryScanUpdate {
  const LibraryScanCompleted({
    required this.assetCount,
    required this.issueCount,
    required this.catalogPath,
    required this.wasLimited,
  });

  final int assetCount;
  final int issueCount;
  final String catalogPath;
  final bool wasLimited;
}

class LibraryScanCancelled extends LibraryScanUpdate {
  const LibraryScanCancelled({
    required this.acceptedItems,
    required this.issueCount,
  });

  final int acceptedItems;
  final int issueCount;
}

class LibraryScanPaused extends LibraryScanUpdate {
  const LibraryScanPaused({
    required this.visitedEntries,
    required this.acceptedItems,
    required this.issueCount,
  });

  final int visitedEntries;
  final int acceptedItems;
  final int issueCount;
}

class LibraryScanStale extends LibraryScanUpdate {
  const LibraryScanStale({
    required this.acceptedItems,
    required this.issueCount,
  });

  final int acceptedItems;
  final int issueCount;
}

class LibraryScanFailure implements Exception {
  const LibraryScanFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
