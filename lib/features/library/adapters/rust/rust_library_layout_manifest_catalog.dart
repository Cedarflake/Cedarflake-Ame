import "dart:typed_data";
import "../../../../src/rust/api/catalog.dart" as rust_api;
import "../../../../src/rust/domain.dart" as rust_domain;
import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";
import "library_gallery_query_mapping.dart";

import "../../application/ports/library_layout_manifest_catalog.dart";

class RustLibraryGalleryLayoutManifestCatalog
    implements LibraryGalleryLayoutManifestCatalog {
  const RustLibraryGalleryLayoutManifestCatalog();

  @override
  Future<LibraryGalleryLayoutManifestChunk> loadChunk({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryGalleryLayoutManifestCursor? after,
  }) async {
    try {
      final chunk = await rust_api.loadLibraryGalleryLayoutManifestChunk(
        maxItems: maxItems,
        query: mapLibraryGalleryQueryToRust(query),
        after: after == null ? null : _mapCursorToRust(after),
      );
      return _mapChunk(chunk);
    } on LibraryGalleryLayoutManifestFailure {
      rethrow;
    } on Object catch (error) {
      if (error case rust_domain.ScanError(:final code, :final message)) {
        throw LibraryGalleryLayoutManifestFailure(code: code, message: message);
      }
      throw LibraryGalleryLayoutManifestFailure(
        code: "bridge_gallery_layout_manifest_load_failed",
        message: error.toString(),
      );
    }
  }

  LibraryGalleryLayoutManifestChunk _mapChunk(
    rust_domain.GalleryLayoutManifestChunk chunk,
  ) {
    final totalItems = _mapCount(chunk.totalItems, "total item count");
    final startOrdinal = _mapCount(chunk.startOrdinal, "start ordinal");
    return LibraryGalleryLayoutManifestChunk(
      revision: chunk.revision,
      queryId: chunk.queryId,
      totalItems: totalItems,
      startOrdinal: startOrdinal,
      locationIds: chunk.locationIds,
      aspectRatioMilli: Uint16List.fromList(chunk.aspectRatioMilli),
      dateGroupIndices: Uint16List.fromList(chunk.dateGroupIndices),
      dateGroups: [for (final group in chunk.dateGroups) group.dateKey],
      flags: Uint8List.fromList(chunk.flags),
      nextCursor: chunk.nextCursor == null
          ? null
          : _mapCursorFromRust(chunk.nextCursor!),
    );
  }

  rust_domain.GalleryLayoutManifestCursor _mapCursorToRust(
    LibraryGalleryLayoutManifestCursor cursor,
  ) {
    return rust_domain.GalleryLayoutManifestCursor(
      revision: cursor.revision,
      queryId: cursor.queryId,
      totalItems: BigInt.from(cursor.totalItems),
      nextOrdinal: BigInt.from(cursor.nextOrdinal),
      after: rust_domain.CatalogCursor(
        revision: cursor.after.revision,
        queryId: cursor.after.queryId,
        primaryMissing: cursor.after.primaryMissing,
        primaryText: cursor.after.primaryText,
        primaryNumber: cursor.after.primaryNumber,
        rootId: cursor.after.rootId,
        locationId: cursor.after.locationId,
      ),
    );
  }

  LibraryGalleryLayoutManifestCursor _mapCursorFromRust(
    rust_domain.GalleryLayoutManifestCursor cursor,
  ) {
    return LibraryGalleryLayoutManifestCursor(
      revision: cursor.revision,
      queryId: cursor.queryId,
      totalItems: _mapCount(cursor.totalItems, "cursor total item count"),
      nextOrdinal: _mapCount(cursor.nextOrdinal, "cursor next ordinal"),
      after: LibraryCatalogCursor(
        revision: cursor.after.revision,
        queryId: cursor.after.queryId,
        primaryMissing: cursor.after.primaryMissing,
        primaryText: cursor.after.primaryText,
        primaryNumber: cursor.after.primaryNumber,
        rootId: cursor.after.rootId,
        locationId: cursor.after.locationId,
      ),
    );
  }

  int _mapCount(BigInt value, String label) {
    if (value.isNegative || value > BigInt.from(0x7fffffffffffffff)) {
      throw LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_count_invalid",
        message: "The $label is outside the supported Dart range",
      );
    }
    return value.toInt();
  }
}
