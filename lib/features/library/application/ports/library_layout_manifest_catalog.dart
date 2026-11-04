import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";

abstract interface class LibraryGalleryLayoutManifestCatalog {
  Future<LibraryGalleryLayoutManifestChunk> loadChunk({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryGalleryLayoutManifestCursor? after,
  });
}

abstract interface class LibraryGalleryLayoutManifestLoader {
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  });
}
