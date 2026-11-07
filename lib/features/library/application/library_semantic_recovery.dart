import "../domain/library_classification_review.dart";
import "../domain/library_models.dart";
import "../domain/library_semantic_search.dart";
import "library_classification_lifecycle.dart";
import "ports/library_semantic_search.dart";

class LibrarySemanticRecovery implements LibrarySemanticSearch {
  LibrarySemanticRecovery(this._service, this._lifecycle);

  final LibrarySemanticSearch _service;
  final LibraryClassificationLifecycle _lifecycle;
  Future<void>? _recovery;

  @override
  Future<LibrarySemanticSearchRequest> prepare(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  ) async {
    await _recovery;
    try {
      if (!await _lifecycle.waitUntilOpened()) {
        throw const LibrarySemanticFailure("semantic_model_unavailable");
      }
      return await _service.prepare(scope, input);
    } on LibraryClassificationFailure catch (failure) {
      throw LibrarySemanticFailure(failure.code);
    } on LibrarySemanticFailure catch (failure) {
      final cause = _lifecycle.error;
      if (failure.code == "semantic_session_unavailable" &&
          cause is LibraryClassificationFailure) {
        throw LibrarySemanticFailure(cause.code);
      }
      rethrow;
    }
  }

  @override
  Future<LibrarySemanticIndexStatus?> indexStatus() async {
    await _recovery;
    return _service.indexStatus();
  }

  @override
  Future<void> retryIndex() => _recovery ??= _lifecycle
      .changeConfiguration(_service.retryIndex)
      .whenComplete(() => _recovery = null);
}
