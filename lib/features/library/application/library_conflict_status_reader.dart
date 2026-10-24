import "dart:async";

import "../domain/library_organization_conflicts.dart";

class LibraryConflictStatusReader {
  LibraryConflictStatusReader(this.load);
  final Future<LibraryConflictStatus> Function() load;
  final _changes = StreamController<LibraryConflictStatus>();
  BigInt? _revision;
  bool _pending = false;
  bool _running = false;
  bool _closed = false;

  Stream<LibraryConflictStatus> get changes => _changes.stream;

  void refresh(BigInt? revision) {
    if (_closed || revision == null) return;
    _revision = revision;
    _pending = true;
    if (!_running) unawaited(_read());
  }

  Future<void> _read() async {
    _running = true;
    try {
      while (_pending && !_closed) {
        _pending = false;
        try {
          final status = await load();
          if (_closed) return;
          if (status.revision >= _revision!) {
            _changes.add(status);
          } else if (!_pending) {
            _changes.addError(
              StateError(
                "Conflict status precedes the requested catalog revision",
              ),
            );
          }
        } on Object catch (error, stack) {
          if (!_closed) _changes.addError(error, stack);
        }
      }
    } finally {
      _running = false;
    }
  }

  void close() {
    _closed = true;
    unawaited(_changes.close());
  }
}
