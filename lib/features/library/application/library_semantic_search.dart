import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_models.dart";
import "../domain/library_semantic_search.dart";
import "ports/library_semantic_search.dart";

final librarySemanticSearchProvider = Provider<LibrarySemanticSearch?>(
  (ref) => throw StateError(
    "LibrarySemanticSearch must be bound during application startup",
  ),
);

class LibrarySemanticSearchSession {
  LibrarySemanticSearchSession(this.service);
  final LibrarySemanticSearch service;
  LibrarySemanticSearchRequest? _active;
  int _generation = 0;
  bool _disposed = false;

  Future<LibrarySemanticResults?> search(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  ) async {
    cancel();
    final generation = _generation;
    final request = await service.prepare(scope, input);
    if (_disposed || generation != _generation) {
      request.cancel();
      request.dispose();
      return null;
    }
    _active = request;
    try {
      final result = await request.run();
      return !_disposed && generation == _generation ? result : null;
    } on Object {
      if (_disposed || generation != _generation) return null;
      rethrow;
    } finally {
      if (identical(_active, request)) _active = null;
      request.dispose();
    }
  }

  void cancel() {
    _generation += 1;
    _active?.cancel();
  }

  void dispose() {
    _disposed = true;
    cancel();
  }
}
