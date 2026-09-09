import "../domain/library_models.dart";
import "library_catalog.dart";

class LibraryQuerySnapshotReader {
  const LibraryQuerySnapshotReader(this._catalog);

  final LibraryCatalog _catalog;

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
    if (snapshot.revision != result.timeline.revision ||
        snapshot.queryId != result.timeline.queryId) {
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
