import "../domain/library_query_snapshot.dart";
import "library_query_update.dart";

typedef LibraryQueryPublication =
    Future<LibraryQueryUpdateOutcome> Function(LibraryQueryAnchor? anchor);

abstract interface class LibraryQueryProjection {
  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication,
  );
}

class LibraryQueryProjectionRegistration {
  LibraryQueryProjectionRegistration._(this._projection);

  final LibraryQueryProjection _projection;
}

class LibraryQueryProjectionRead {
  LibraryQueryProjectionRead._(this._owner, this._generation);

  final LibraryQueryProjections _owner;
  final int _generation;
}

/// A mounted gallery may retain its position around an application-owned read.
/// Detaching an old gallery cannot revoke its replacement's registration.
class LibraryQueryProjections {
  LibraryQueryProjectionRegistration? _current;
  int _generation = 0;
  bool _isDisposed = false;

  int get generation => _generation;

  LibraryQueryProjectionRead beginRead() =>
      LibraryQueryProjectionRead._(this, _generation);

  bool accepts(LibraryQueryProjectionRead read) =>
      !_isDisposed &&
      identical(read._owner, this) &&
      read._generation == _generation;

  void invalidatePosition() => _generation += 1;

  LibraryQueryProjectionRegistration attach(LibraryQueryProjection projection) {
    final registration = LibraryQueryProjectionRegistration._(projection);
    if (!_isDisposed) {
      invalidatePosition();
      _current = registration;
    }
    return registration;
  }

  void detach(LibraryQueryProjectionRegistration registration) {
    if (identical(_current, registration)) {
      invalidatePosition();
      _current = null;
    }
  }

  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication,
  ) {
    if (_isDisposed) {
      return Future.value(LibraryQueryUpdateOutcome.superseded);
    }
    final current = _current;
    return current == null
        ? publication(null)
        : current._projection.publish(publication);
  }

  void dispose() {
    _isDisposed = true;
    _current = null;
  }
}
