import "dart:async";

import "../domain/library_file_operations.dart";

/// Keeps native progress and the final gallery publication in one operation.
class LibraryFilePublication {
  LibraryFilePublication(this._refresh);

  final Future<void> Function() _refresh;
  final _interval = Stopwatch()..start();
  Future<void>? _running;
  ({Object error, StackTrace stack})? _failure;
  int _observed = 0;
  int _requested = 0;

  void observe(LibraryFileProgress progress) {
    if (progress.published > _observed) _observed = progress.published;
    if (_failure == null &&
        _running == null &&
        _observed > _requested &&
        (progress.isFinished ||
            _interval.elapsed >= const Duration(milliseconds: 500))) {
      _start();
    }
  }

  void _start() {
    _requested = _observed;
    _running = _read();
  }

  Future<void> _read() async {
    try {
      await Future<void>.sync(_refresh);
    } on Object catch (error, stack) {
      _failure = (error: error, stack: stack);
    } finally {
      _running = null;
      _interval.reset();
    }
  }

  Future<void> finish() async {
    await _running;
    if (_observed > _requested && _failure == null) {
      _start();
      await _running;
    }
    if (_failure case final failure?) {
      Error.throwWithStackTrace(failure.error, failure.stack);
    }
  }

  Future<void> close() async {
    await _running;
    _interval.stop();
  }
}
