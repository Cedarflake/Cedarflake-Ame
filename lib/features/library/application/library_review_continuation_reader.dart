import "../domain/library_models.dart";
import "../domain/library_review.dart";

class LibraryContinuationSnapshot {
  const LibraryContinuationSnapshot.ready(this.query, this.value)
    : failed = false;
  const LibraryContinuationSnapshot.failed(this.query)
    : value = null,
      failed = true;

  final LibraryGalleryQuery query;
  final LibraryReviewContinuation? value;
  final bool failed;
}

/// Admits completed same-query reads while coalescing later catalog publications.
class LibraryReviewContinuationReader {
  LibraryReviewContinuationReader(this.load, this.onChanged);

  final Future<LibraryReviewContinuation> Function(LibraryGalleryQuery) load;
  final void Function() onChanged;
  LibraryContinuationSnapshot? latest;
  LibraryGalleryQuery? _requested;
  Future<void>? _running;
  int _scope = 0;
  bool _pending = false;
  bool _closed = false;

  Future<void> refresh(LibraryGalleryQuery query) {
    if (_closed) return Future.value();
    if (_requested != query) _scope++;
    _requested = query;
    _pending = true;
    return _running ??= _read();
  }

  Future<void> _read() async {
    try {
      while (_pending && !_closed) {
        _pending = false;
        final query = _requested!;
        final scope = _scope;
        LibraryContinuationSnapshot result;
        try {
          result = LibraryContinuationSnapshot.ready(query, await load(query));
        } on Object {
          result = LibraryContinuationSnapshot.failed(query);
        }
        if (_closed) return;
        if (scope != _scope) continue;
        latest = result;
        onChanged();
      }
    } finally {
      _running = null;
    }
  }

  void invalidate() {
    _scope++;
    _requested = null;
    _pending = false;
    latest = null;
  }

  void close() {
    _closed = true;
    invalidate();
  }
}
