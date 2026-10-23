import "dart:async";

/// Serializes root-window reads and replaces superseded queued refreshes.
class LibraryFolderRootLoader {
  LibraryFolderRootLoader(this.loadRoot);

  final Future<void> Function(BigInt revision, String rootId) loadRoot;
  ({BigInt revision, List<String> rootIds})? _pending;
  Future<void>? _reading;
  bool _isDisposed = false;

  void refresh(BigInt revision, Iterable<String> rootIds) {
    if (_isDisposed) {
      return;
    }
    _pending = (revision: revision, rootIds: List.unmodifiable(rootIds));
    _reading ??= Future<void>.microtask(_drain);
  }

  Future<void> _drain() async {
    try {
      while (!_isDisposed && _pending != null) {
        final request = _pending!;
        _pending = null;
        for (final rootId in request.rootIds) {
          if (_isDisposed || _pending != null) {
            break;
          }
          await loadRoot(request.revision, rootId);
        }
      }
    } finally {
      _reading = null;
    }
  }

  void dispose() {
    _isDisposed = true;
    _pending = null;
  }
}
