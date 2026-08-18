import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/catalog.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";

const libraryCatalogWindow = 500;
const libraryTimelineWindow = 160;
const libraryFolderWindow = 200;

abstract interface class LibraryCatalog {
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  });

  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query);

  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  });

  Future<bool> unregisterRoot(String rootId);
}

abstract interface class LibraryQueryAnchorCatalog {
  Future<LibrarySnapshot> loadAroundLocation({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String anchorLocationId,
  });
}

abstract interface class LibraryStableQueryAnchorCatalog {
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  });
}

abstract interface class LibraryStableAssetCatalog {
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  });
}

abstract interface class LibraryFolderCatalog {
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  });
}

class RustLibraryCatalog
    implements
        LibraryCatalog,
        LibraryFolderCatalog,
        LibraryQueryAnchorCatalog,
        LibraryStableQueryAnchorCatalog,
        LibraryStableAssetCatalog {
  const RustLibraryCatalog();

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    try {
      final Future<rust_domain.CatalogSnapshot> pendingSnapshot = rust_api
          .loadLibraryCatalog(
            maxItems: maxItems,
            query: _mapQuery(query),
            after: after == null
                ? null
                : rust_domain.CatalogCursor(
                    revision: after.revision,
                    queryId: after.queryId,
                    primaryMissing: after.primaryMissing,
                    primaryText: after.primaryText,
                    primaryNumber: after.primaryNumber,
                    rootId: after.rootId,
                    locationId: after.locationId,
                  ),
            before: before == null
                ? null
                : rust_domain.CatalogCursor(
                    revision: before.revision,
                    queryId: before.queryId,
                    primaryMissing: before.primaryMissing,
                    primaryText: before.primaryText,
                    primaryNumber: before.primaryNumber,
                    rootId: before.rootId,
                    locationId: before.locationId,
                  ),
          );
      final snapshot = await pendingSnapshot;
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_catalog_load_failed");
    }
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    try {
      final timeline = rust_api.loadLibraryGalleryTimeline(
        query: _mapQuery(query),
      );
      return LibraryTimeline(
        revision: timeline.revision,
        queryId: timeline.queryId,
        totalItems: timeline.totalItems.toInt(),
        buckets: List.unmodifiable(
          timeline.buckets.map(
            (bucket) => LibraryTimeBucket(
              monthKey: bucket.monthKey,
              itemCount: bucket.itemCount.toInt(),
              aspectRatioSum: bucket.aspectRatioMilliSum.toInt() / 1000,
            ),
          ),
        ),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_timeline_load_failed");
    }
  }

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    try {
      final page = rust_api.loadLibraryFolderPage(
        rootId: rootId,
        parentRelativePath: parentRelativePath,
        maxItems: maxItems,
        after: after == null
            ? null
            : rust_domain.LibraryFolderCursor(
                revision: after.revision,
                rootId: after.rootId,
                parentRelativePath: after.parentRelativePath,
                relativePath: after.relativePath,
              ),
      );
      final nextCursor = page.nextCursor;
      return LibraryFolderPage(
        revision: page.revision,
        rootId: page.rootId,
        parentRelativePath: page.parentRelativePath,
        folders: List.unmodifiable(
          page.folders.map(
            (folder) => LibraryFolder(
              rootId: folder.rootId,
              relativePath: folder.relativePath,
              name: folder.name,
              directAssetCount: folder.directAssetCount.toInt(),
              descendantAssetCount: folder.descendantAssetCount.toInt(),
            ),
          ),
        ),
        nextCursor: nextCursor == null
            ? null
            : LibraryFolderCursor(
                revision: nextCursor.revision,
                rootId: nextCursor.rootId,
                parentRelativePath: nextCursor.parentRelativePath,
                relativePath: nextCursor.relativePath,
              ),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_folder_page_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    try {
      final Future<rust_domain.CatalogSnapshot> pendingSnapshot = rust_api
          .loadLibraryCatalogAtTime(
            maxItems: maxItems,
            query: _mapQuery(query),
            anchor: rust_domain.GalleryTimeAnchor(
              revision: anchor.revision,
              queryId: anchor.queryId,
              monthKey: anchor.monthKey,
              itemOffset: BigInt.from(anchor.itemOffset),
            ),
          );
      final snapshot = await pendingSnapshot;
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_time_anchor_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAroundLocation({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String anchorLocationId,
  }) async {
    try {
      final snapshot = await rust_api.loadLibraryCatalogAroundLocation(
        maxItems: maxItems,
        query: _mapQuery(query),
        anchorLocationId: anchorLocationId,
      );
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_location_anchor_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    try {
      final snapshot = await rust_api.loadLibraryCatalogAroundAsset(
        maxItems: maxItems,
        query: _mapQuery(query),
        requestedLocationId: requestedLocationId,
        anchorAssetId: anchorAssetId,
        fallbackOrdinal: BigInt.from(fallbackGlobalItemIndex),
      );
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_asset_anchor_load_failed");
    }
  }

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) async {
    try {
      final asset = await rust_api.loadLibraryAssetById(
        assetId: assetId,
        preferredLocationId: preferredLocationId,
      );
      return asset == null ? null : mapRustLibraryAsset(asset);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_asset_identity_load_failed");
    }
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    try {
      return rust_api.removeLibraryRoot(rootId: rootId);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_root_unregister_failed");
    }
  }

  LibrarySnapshot _mapSnapshot(rust_domain.CatalogSnapshot snapshot) {
    return LibrarySnapshot(
      catalogPath: snapshot.catalogPath,
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      roots: List.unmodifiable(snapshot.roots.map(_mapRoot)),
      assets: List.unmodifiable(snapshot.assets.map(mapRustLibraryAsset)),
      previousCursor: _mapCursor(snapshot.previousCursor),
      nextCursor: _mapCursor(snapshot.nextCursor),
      queryAnchorResolution: snapshot.queryAnchorResolution == null
          ? null
          : LibraryQueryAnchorResolution(
              requestedLocationId:
                  snapshot.queryAnchorResolution!.requestedLocationId,
              locationId: snapshot.queryAnchorResolution!.locationId,
              ordinal: snapshot.queryAnchorResolution!.ordinal?.toInt(),
              windowStartItemOffset: snapshot
                  .queryAnchorResolution!
                  .windowStartOrdinal
                  .toInt(),
            ),
    );
  }

  LibraryCatalogFailure _mapFailure(Object error, String fallbackCode) {
    if (error case rust_domain.ScanError(:final code, :final message)) {
      return LibraryCatalogFailure(code: code, message: message);
    }
    return LibraryCatalogFailure(code: fallbackCode, message: error.toString());
  }

  LibraryCatalogCursor? _mapCursor(rust_domain.CatalogCursor? cursor) {
    if (cursor == null) {
      return null;
    }
    return LibraryCatalogCursor(
      revision: cursor.revision,
      queryId: cursor.queryId,
      primaryMissing: cursor.primaryMissing,
      primaryText: cursor.primaryText,
      primaryNumber: cursor.primaryNumber,
      rootId: cursor.rootId,
      locationId: cursor.locationId,
    );
  }

  rust_domain.GalleryQuery _mapQuery(LibraryGalleryQuery query) {
    return mapLibraryGalleryQueryToRust(query);
  }

  LibraryRoot _mapRoot(rust_domain.LibraryRootView root) {
    return LibraryRoot(
      id: root.rootId,
      path: root.path,
      displayPath: root.displayPath,
      activeScanId: root.activeScanId,
      createdUnixMs: root.createdUnixMs,
      assetCount: root.assetCount.toInt(),
      issueCount: root.issueCount.toInt(),
      availability: switch (root.availability) {
        rust_domain.LibraryRootAvailability.unknown =>
          LibraryRootAvailability.unknown,
        rust_domain.LibraryRootAvailability.available =>
          LibraryRootAvailability.available,
        rust_domain.LibraryRootAvailability.missing =>
          LibraryRootAvailability.missing,
        rust_domain.LibraryRootAvailability.inaccessible =>
          LibraryRootAvailability.inaccessible,
        rust_domain.LibraryRootAvailability.offline =>
          LibraryRootAvailability.offline,
      },
      availabilityMessage: root.availabilityMessage,
    );
  }
}

rust_domain.GalleryQuery mapLibraryGalleryQueryToRust(
  LibraryGalleryQuery query,
) {
  return rust_domain.GalleryQuery(
    rootId: query.rootId,
    folderRelativePath: query.folderRelativePath,
    includeDescendants: query.includeDescendants,
    searchText: query.searchText,
    sortKey: switch (query.sortKey) {
      LibraryGallerySortKey.captureTime =>
        rust_domain.GallerySortKey.captureTime,
      LibraryGallerySortKey.createdTime =>
        rust_domain.GallerySortKey.createdTime,
      LibraryGallerySortKey.modifiedTime =>
        rust_domain.GallerySortKey.modifiedTime,
      LibraryGallerySortKey.fileName => rust_domain.GallerySortKey.fileName,
    },
    sortDirection: switch (query.sortDirection) {
      LibraryGallerySortDirection.ascending =>
        rust_domain.GallerySortDirection.ascending,
      LibraryGallerySortDirection.descending =>
        rust_domain.GallerySortDirection.descending,
    },
  );
}

LibraryAsset mapRustLibraryAsset(rust_domain.AssetLocationView asset) {
  final captureTime = asset.captureTime;
  final fileIdentity = asset.fileIdentity;
  return LibraryAsset(
    assetId: asset.assetId,
    locationId: asset.locationId,
    rootId: asset.rootId,
    sourcePath: asset.absolutePath,
    displayPath: asset.displayPath,
    relativePath: asset.relativePath,
    previewPath: asset.previewPath,
    fileSize: asset.fileSize,
    createdUnixMs: asset.createdUnixMs,
    modifiedUnixMs: asset.modifiedUnixMs,
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

class LibraryCatalogFailure implements Exception {
  const LibraryCatalogFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}

final libraryCatalogProvider = Provider<LibraryCatalog>((ref) {
  return const RustLibraryCatalog();
});

final libraryFolderCatalogProvider = Provider<LibraryFolderCatalog>((ref) {
  return const RustLibraryCatalog();
});
