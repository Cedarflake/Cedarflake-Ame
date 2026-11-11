import "dart:async";

import "../domain/library_classification_configuration.dart";

/// Publishes each completed category read and coalesces intervening catalog updates.
class LibraryClassificationNavigationReader {
  LibraryClassificationNavigationReader(this.load);

  final Future<LibraryClassificationSettings> Function() load;
  final _changes = StreamController<LibraryClassificationSettings>();
  bool _pending = false;
  bool _reading = false;
  bool _closed = false;

  Stream<LibraryClassificationSettings> get changes => _changes.stream;

  void refresh() {
    if (_closed) return;
    _pending = true;
    if (!_reading) unawaited(_read());
  }

  Future<void> _read() async {
    _reading = true;
    try {
      while (_pending && !_closed) {
        _pending = false;
        try {
          final result = await load();
          if (!_closed) _changes.add(result);
        } on Object catch (error, stackTrace) {
          if (!_closed) _changes.addError(error, stackTrace);
        }
      }
    } finally {
      _reading = false;
    }
  }

  void close() {
    _closed = true;
    unawaited(_changes.close());
  }
}
