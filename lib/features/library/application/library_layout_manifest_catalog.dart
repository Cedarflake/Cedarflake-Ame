import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/gallery_layout_manifest.dart";
import "../domain/library_models.dart";
import "ports/library_layout_manifest_catalog.dart";

const libraryGalleryLayoutManifestChunkSize = 4096;

class LibraryGalleryLayoutManifestRequest {
  const LibraryGalleryLayoutManifestRequest({
    required this.query,
    required this.revision,
    required this.queryId,
  });

  final LibraryGalleryQuery query;
  final BigInt revision;
  final String queryId;

  @override
  int get hashCode => Object.hash(query, revision, queryId);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is LibraryGalleryLayoutManifestRequest &&
            query == other.query &&
            revision == other.revision &&
            queryId == other.queryId;
  }
}

class CatalogLibraryGalleryLayoutManifestLoader
    implements LibraryGalleryLayoutManifestLoader {
  const CatalogLibraryGalleryLayoutManifestLoader(this._catalog);

  final LibraryGalleryLayoutManifestCatalog _catalog;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async {
    LibraryGalleryLayoutManifestBuilder? builder;
    LibraryGalleryLayoutManifestCursor? cursor;
    while (true) {
      _throwIfCancelled(isCancelled);
      final chunk = await _catalog.loadChunk(
        maxItems: libraryGalleryLayoutManifestChunkSize,
        query: query,
        after: cursor,
      );
      _throwIfCancelled(isCancelled);
      builder ??= LibraryGalleryLayoutManifestBuilder(
        revision: chunk.revision,
        queryId: chunk.queryId,
        totalItems: chunk.totalItems,
        hasGroupIdentity: chunk.entryIds != null,
      );
      builder.append(chunk);
      cursor = chunk.nextCursor;
      if (cursor == null) {
        return builder.build();
      }
    }
  }

  void _throwIfCancelled(bool Function()? isCancelled) {
    if (isCancelled?.call() ?? false) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_cancelled",
        message: "The gallery layout manifest request was cancelled",
      );
    }
  }
}

final libraryGalleryLayoutManifestCatalogProvider =
    Provider<LibraryGalleryLayoutManifestCatalog>((ref) {
      throw StateError(
        "LibraryGalleryLayoutManifestCatalog must be bound during application startup",
      );
    });

final libraryGalleryLayoutManifestLoaderProvider =
    Provider<LibraryGalleryLayoutManifestLoader>((ref) {
      return CatalogLibraryGalleryLayoutManifestLoader(
        ref.watch(libraryGalleryLayoutManifestCatalogProvider),
      );
    });

final libraryGalleryLayoutManifestProvider = FutureProvider.autoDispose
    .family<LibraryGalleryLayoutManifest, LibraryGalleryLayoutManifestRequest>((
      ref,
      request,
    ) async {
      var isCancelled = false;
      ref.onDispose(() {
        isCancelled = true;
      });
      final manifest = await ref
          .watch(libraryGalleryLayoutManifestLoaderProvider)
          .load(request.query, isCancelled: () => isCancelled);
      if (manifest.revision != request.revision ||
          manifest.queryId != request.queryId) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_stale",
          message: "The completed gallery layout manifest is no longer active",
        );
      }
      return manifest;
    }, retry: (_, _) => null);
