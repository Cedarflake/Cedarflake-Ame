import "dart:async";

import "../domain/library_models.dart";
import "library_source_reader.dart";

/// Owns one active read and the latest waiting viewer intent, without closing an
/// active lease before the native buffer copy that uses it has finished.
class LibrarySourceReadScheduler {
  LibrarySourceReadScheduler({required this._reader});

  final LibrarySourceReader _reader;
  LibrarySourceReadRequest? _active;
  LibrarySourceReadRequest? _pending;

  LibrarySourceReadRequest request(LibraryAsset asset) {
    _active?.cancel();
    _pending?.cancel();
    final request = LibrarySourceReadRequest._(asset, this);
    _pending = request;
    _drain();
    return request;
  }

  void _cancel(LibrarySourceReadRequest request) {
    request._isCancelled = true;
    if (identical(_pending, request)) {
      _pending = null;
      request._completion.completeError(_superseded());
    }
  }

  void _drain() {
    if (_active != null) {
      return;
    }
    final request = _pending;
    if (request == null) {
      return;
    }
    _pending = null;
    _active = request;
    unawaited(_acquire(request));
  }

  Future<void> _acquire(LibrarySourceReadRequest request) async {
    try {
      final lease = await _reader.acquire(request.asset);
      if (request.isCancelled) {
        await lease.close();
        throw _superseded();
      }
      request._completion.complete(
        _ScheduledSourceReadLease(lease, () => _finish(request)),
      );
    } on Object catch (error, stackTrace) {
      request._completion.completeError(error, stackTrace);
      _finish(request);
    }
  }

  void _finish(LibrarySourceReadRequest request) {
    if (identical(_active, request)) {
      _active = null;
      _drain();
    }
  }

  static LibrarySourceReadFailure _superseded() =>
      const LibrarySourceReadFailure(
        "viewer_source_superseded",
        "A newer viewer selection replaced this source read",
      );
}

class LibrarySourceReadRequest {
  LibrarySourceReadRequest._(this.asset, this._owner) {
    // Cancellation can precede the image stream attaching its awaiter. Observe
    // the original future immediately without changing its result for callers.
    unawaited(
      _completion.future.then<void>(
        (_) {},
        onError: (Object _, StackTrace _) {},
      ),
    );
  }

  final LibraryAsset asset;
  final LibrarySourceReadScheduler _owner;
  final Completer<LibrarySourceReadLease> _completion = Completer();
  bool _isCancelled = false;

  Future<LibrarySourceReadLease> get lease => _completion.future;
  bool get isCancelled => _isCancelled;

  void cancel() => _owner._cancel(this);
}

class _ScheduledSourceReadLease implements LibrarySourceReadLease {
  _ScheduledSourceReadLease(this._inner, this._onClosed);

  final LibrarySourceReadLease _inner;
  final void Function() _onClosed;
  Future<void>? _closing;

  @override
  String get sourcePath => _inner.sourcePath;

  @override
  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    try {
      await _inner.close();
    } finally {
      _onClosed();
    }
  }
}
