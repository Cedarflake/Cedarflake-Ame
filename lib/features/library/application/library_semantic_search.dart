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
  Future<void>? _preparing;
  int _generation = 0;
  bool _disposed = false;

  Future<LibrarySemanticResults?> search(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  ) async {
    cancel();
    final generation = _generation;
    // Native preparation replaces its pending request on arrival. Admit handles
    // in order so an older bridge call cannot cancel a newer search.
    await _preparing;
    if (_disposed || generation != _generation) return null;
    final preparation = Future<LibrarySemanticSearchRequest>.sync(
      () => service.prepare(scope, input),
    );
    final settled = preparation.then<void>(
      (_) {},
      onError: (Object _, StackTrace _) {},
    );
    _preparing = settled;
    late final LibrarySemanticSearchRequest request;
    try {
      request = await preparation;
    } on Object {
      if (_disposed || generation != _generation) return null;
      rethrow;
    } finally {
      if (identical(_preparing, settled)) _preparing = null;
    }
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
