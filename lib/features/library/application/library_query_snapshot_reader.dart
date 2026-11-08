import "../domain/library_models.dart";
import "library_catalog.dart";
import "ports/library_catalog.dart";
import "../domain/library_query_snapshot.dart";

class LibraryQuerySnapshotReader {
  const LibraryQuerySnapshotReader(this._catalog);

  final LibraryCatalog _catalog;

  Future<LibraryQuerySnapshot?> loadCurrent({
    required LibraryGalleryQuery query,
    required LibraryQueryAnchor? anchor,
    required bool Function() canPublish,
    BigInt? minimumRevision,
  }) async {
    if (!canPublish()) {
      return null;
    }
    try {
      final result = await load(
        query: query,
        anchor: anchor,
        minimumRevision: minimumRevision,
      );
      return canPublish() ? result : null;
    } on Object {
      if (!canPublish()) {
        return null;
      }
      rethrow;
    }
  }

  Future<LibraryQuerySnapshot> load({
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
    BigInt? minimumRevision,
  }) async {
    final result = await _catalog.loadQuerySnapshot(
      maxItems: libraryCatalogWindow,
      query: query,
      anchor: anchor,
    );
    final snapshot = result.snapshot;
    final layout = result.layoutManifest;
    if (snapshot.revision != result.timeline.revision ||
        snapshot.queryId != result.timeline.queryId ||
        (layout != null &&
            (layout.revision != snapshot.revision ||
                layout.queryId != snapshot.queryId ||
                layout.itemCount != result.timeline.totalItems))) {
      throw const LibraryCatalogFailure(
        code: "catalog_query_snapshot_invalid",
        message:
            "The catalog query returned inconsistent page and timeline evidence",
      );
    }
    if (minimumRevision != null && snapshot.revision < minimumRevision) {
      throw const LibraryCatalogFailure(
        code: "catalog_revision_stale",
        message: "The catalog revision has not reached the requested refresh",
      );
    }
    return result;
  }
}
