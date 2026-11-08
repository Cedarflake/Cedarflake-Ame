import "../domain/library_models.dart";
import "ports/library_catalog.dart";
import "../domain/library_query_snapshot.dart";
import "library_query_snapshot_reader.dart";

class LibraryRootRemovalPageReader {
  const LibraryRootRemovalPageReader(this._catalog);

  final LibraryCatalog _catalog;

  Future<LibraryQuerySnapshot?> loadCurrent({
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

  Future<LibraryQuerySnapshot> load({
    required String removedRootId,
    required LibraryGalleryQuery query,
    required LibraryQueryAnchor? anchor,
  }) async {
    final result = await LibraryQuerySnapshotReader(
      _catalog,
    ).load(query: query, anchor: anchor);
    if (result.snapshot.roots.any((root) => root.id == removedRootId)) {
      throw const LibraryCatalogFailure(
        code: "catalog_root_removal_not_visible",
        message: "The removed library root is still present in the catalog",
      );
    }
    return result;
  }
}
