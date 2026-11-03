import "../../domain/library_exact_duplicates.dart";

abstract interface class LibraryGalleryDuplicates {
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  );
}
