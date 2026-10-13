import "dart:async";

class LibraryCatalogPublicationLease {
  LibraryCatalogPublicationLease._();

  final Completer<void> _released = Completer<void>();
}

/// Serializes exclusive catalog projections without blocking independent scans.
class LibraryCatalogPublicationCoordinator {
  LibraryCatalogPublicationLease? _owner;
  int _generation = 0;
  bool _isDisposed = false;

  bool get isReserved => _owner != null;
  int get generation => _generation;

  LibraryCatalogPublicationLease? reserve() {
    if (_isDisposed || _owner != null) {
      return null;
    }
    _generation += 1;
    return _owner = LibraryCatalogPublicationLease._();
  }

  void release(LibraryCatalogPublicationLease lease) {
    if (!identical(_owner, lease)) {
      return;
    }
    _owner = null;
    lease._released.complete();
  }

  Future<bool> waitUntilAvailable() async {
    while (!_isDisposed) {
      final owner = _owner;
      if (owner == null) {
        return true;
      }
      await owner._released.future;
    }
    return false;
  }

  void dispose() {
    _isDisposed = true;
    final owner = _owner;
    if (owner != null) {
      release(owner);
    }
  }
}
