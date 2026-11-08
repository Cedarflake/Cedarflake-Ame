import "../../domain/library_models.dart";
import "../../domain/library_semantic_search.dart";

abstract interface class LibrarySemanticSearch {
  Future<LibrarySemanticSearchRequest> prepare(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  );
  Future<LibrarySemanticIndexStatus?> indexStatus();
  Future<void> retryIndex();
}

abstract interface class LibrarySemanticSearchRequest {
  Future<LibrarySemanticResults> run();
  void cancel();
  void dispose();
}
