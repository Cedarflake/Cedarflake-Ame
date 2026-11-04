import "../../domain/library_models.dart";
import "../../domain/library_folder_models.dart";
import "../../domain/library_query_snapshot.dart";
import "../../domain/library_time_snapshot.dart";

abstract interface class LibraryCatalog {
  Future<LibraryTimeSnapshot> resolveTimeIntent({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeIntent intent,
  });

  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  });

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

class LibraryCatalogFailure implements Exception {
  const LibraryCatalogFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
