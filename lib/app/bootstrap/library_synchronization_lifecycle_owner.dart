import "dart:async";

import "package:flutter/foundation.dart";

import "../../features/library/application/library_synchronization.dart";

typedef LibrarySynchronizationStartErrorReporter =
    void Function(Object error, StackTrace stackTrace);

class LibrarySynchronizationLifecycleOwner {
  LibrarySynchronizationLifecycleOwner(
    this._synchronization, {
    LibrarySynchronizationStartErrorReporter? reportStartError,
  }) : _reportStartError = reportStartError ?? _debugStartError;

  final LibrarySynchronization _synchronization;
  final LibrarySynchronizationStartErrorReporter _reportStartError;
  Future<LibrarySynchronizationStartResult>? _startOperation;
  Future<void>? _closeOperation;
  bool _didScheduleStart = false;
  bool _isClosing = false;

  Future<LibrarySynchronizationStartResult>? get startOperation =>
      _startOperation;

  void startInBackground() {
    if (_didScheduleStart || _isClosing) {
      return;
    }
    _didScheduleStart = true;
    final completion = Completer<LibrarySynchronizationStartResult>();
    _startOperation = completion.future;
    scheduleMicrotask(() {
      if (_isClosing) {
        completion.complete(LibrarySynchronizationStartResult.skipped);
        return;
      }

      late final Future<LibrarySynchronizationStartResult> start;
      try {
        start = _synchronization.start();
      } on Object catch (error, stackTrace) {
        _handleStartError(error, stackTrace);
        completion.complete(LibrarySynchronizationStartResult.failed);
        return;
      }
      unawaited(
        start.then<void>(
          completion.complete,
          onError: (Object error, StackTrace stackTrace) {
            _handleStartError(error, stackTrace);
            completion.complete(LibrarySynchronizationStartResult.failed);
          },
        ),
      );
    });
  }

  Future<void> close() {
    _isClosing = true;
    final activeOperation = _closeOperation;
    if (activeOperation != null) {
      return activeOperation;
    }
    final operation = Future<void>.sync(_synchronization.dispose);
    late final Future<void> trackedOperation;
    trackedOperation = operation.then<void>(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        if (identical(_closeOperation, trackedOperation)) {
          _closeOperation = null;
        }
        Error.throwWithStackTrace(error, stackTrace);
      },
    );
    _closeOperation = trackedOperation;
    return trackedOperation;
  }

  void _handleStartError(Object error, StackTrace stackTrace) {
    try {
      _reportStartError(error, stackTrace);
    } on Object catch (reportingError) {
      debugPrint(
        "[Ame sync] phase=background-start-error-reporting result=failed "
        "error=${reportingError.runtimeType}",
      );
    }
  }

  static void _debugStartError(Object error, StackTrace stackTrace) {
    debugPrint(
      "[Ame sync] phase=background-start result=failed "
      "error=${error.runtimeType}",
    );
  }
}
