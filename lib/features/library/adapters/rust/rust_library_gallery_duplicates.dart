import "library_duplicate_failure.dart";
import "../../../../src/rust/api/exact_duplicates.dart" as rust_api;
import "../../application/ports/library_gallery_duplicates.dart";
import "../../domain/library_exact_duplicates.dart";

class RustLibraryGalleryDuplicates implements LibraryGalleryDuplicates {
  const RustLibraryGalleryDuplicates();

  @override
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  ) async {
    final page = await invokeDuplicateBridge(
      () => rust_api.loadGalleryDuplicates(
        revision: revision,
        locationIds: locationIds,
      ),
    );
    if (page.revision < revision) {
      throw StateError(
        "Duplicate annotations precede the requested gallery revision",
      );
    }
    return {
      for (final annotation in page.annotations)
        annotation.locationId: LibraryDuplicateGroup(
          digest: annotation.group.digest,
          byteLength: annotation.group.byteLength.toInt(),
          locationCount: annotation.group.locationCount.toInt(),
          physicalFileCount: annotation.group.physicalFileCount.toInt(),
          representativeLocationId: annotation.group.representativeLocationId,
          representativePath: annotation.group.representativePath,
          pendingLocationCount: annotation.pendingLocationCount.toInt(),
        ),
    };
  }
}
