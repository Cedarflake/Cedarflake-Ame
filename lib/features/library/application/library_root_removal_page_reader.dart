import "../domain/library_models.dart";
import "library_catalog.dart";

class LibraryRootRemovalPageReader {
  const LibraryRootRemovalPageReader(this._catalog);

  final LibraryCatalog _catalog;

  Future<LibrarySnapshot?> loadCurrent({
    required String removedRootId,
    required LibraryGalleryQuery query,
    required LibraryQueryAnchor? anchor,
    required bool Function() canPublish,
  }) async {
    if (!canPublish()) {
      return null;
    }
    try {
      final snapshot = await load(
        removedRootId: removedRootId,
        query: query,
        anchor: anchor,
      );
      return canPublish() ? snapshot : null;
    } on Object {
      if (!canPublish()) {
        return null;
      }
      rethrow;
    }
  }

  Future<LibrarySnapshot> load({
    required String removedRootId,
    required LibraryGalleryQuery query,
    required LibraryQueryAnchor? anchor,
  }) async {
    final LibrarySnapshot snapshot;
    final catalog = _catalog;
    if (anchor == null) {
      snapshot = await catalog.load(
        maxItems: libraryCatalogWindow,
        query: query,
      );
    } else if (anchor.assetId != null &&
        catalog is LibraryStableQueryAnchorCatalog) {
      snapshot = await (catalog as LibraryStableQueryAnchorCatalog)
          .loadAroundAsset(
            maxItems: libraryCatalogWindow,
            query: query,
            requestedLocationId: anchor.requestedLocationId,
            anchorAssetId: anchor.assetId!,
            fallbackGlobalItemIndex: anchor.fallbackGlobalItemIndex,
          );
    } else if (catalog is LibraryQueryAnchorCatalog) {
      snapshot = await (catalog as LibraryQueryAnchorCatalog)
          .loadAroundLocation(
            maxItems: libraryCatalogWindow,
            query: query,
            anchorLocationId: anchor.requestedLocationId,
          );
    } else {
      snapshot = (await catalog.loadQuerySnapshot(
        maxItems: libraryCatalogWindow,
        query: query,
        anchor: anchor,
      )).snapshot;
    }
    if (snapshot.roots.any((root) => root.id == removedRootId)) {
      throw const LibraryCatalogFailure(
        code: "catalog_root_removal_not_visible",
        message: "The removed library root is still present in the catalog",
      );
    }
    return snapshot;
  }
}
