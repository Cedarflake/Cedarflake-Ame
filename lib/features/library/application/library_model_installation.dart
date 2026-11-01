import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/legacy.dart";

enum LibraryModelInstallationStage {
  idle,
  preparing,
  downloading,
  stopping,
  enabling,
  enabled,
  cancelled,
  failed,
}

class LibraryModelDownloadProgress {
  const LibraryModelDownloadProgress({
    required this.downloading,
    required this.completedBytes,
    required this.totalBytes,
  });

  final bool downloading;
  final int completedBytes;
  final int totalBytes;
}

abstract interface class LibraryModelInstallationSession {
  Future<void> prepare();
  LibraryModelDownloadProgress? progress();
  void cancel();
  Future<void> commit();
  void dispose();
}

abstract interface class LibraryModelInstallationService {
  LibraryModelInstallationSession create();
}

typedef EnablePreparedLibraryModel =
    Future<void> Function(Future<void> Function() commit);

final libraryModelInstallationProvider =
    ChangeNotifierProvider<LibraryModelInstallation?>((ref) => null);

class LibraryModelInstallation extends ChangeNotifier {
  LibraryModelInstallation(this._service);

  final LibraryModelInstallationService _service;
  LibraryModelInstallationSession? _session;
  Future<void>? _operation;
  Timer? _poll;
  bool _closed = false;
  bool _cancelRequested = false;
  LibraryModelInstallationStage _stage = LibraryModelInstallationStage.idle;
  LibraryModelDownloadProgress? _progress;
  Object? _error;
  int _sequence = 0;
  DateTime? _finishedAt;

  LibraryModelInstallationStage get stage => _stage;
  LibraryModelDownloadProgress? get progress => _progress;
  Object? get error => _error;
  int get sequence => _sequence;
  DateTime? get finishedAt => _finishedAt;
  bool get busy => _operation != null;
  bool get canStart => !_closed && !busy;
  bool get canCancel =>
      busy &&
      (_stage == LibraryModelInstallationStage.preparing ||
          _stage == LibraryModelInstallationStage.downloading);

  Future<void> start(EnablePreparedLibraryModel enable) {
    if (_closed || _operation != null) {
      return Future.error(StateError("Model installation is unavailable"));
    }
    _cancelRequested = false;
    _progress = null;
    _error = null;
    _finishedAt = null;
    _sequence += 1;
    _stage = LibraryModelInstallationStage.preparing;
    final operation = Future<void>.microtask(() => _run(enable));
    _operation = operation;
    notifyListeners();
    return operation;
  }

  Future<void> _run(EnablePreparedLibraryModel enable) async {
    try {
      if (_closed || _cancelRequested) return;
      final session = _service.create();
      _session = session;
      _poll = Timer.periodic(const Duration(milliseconds: 200), (_) {
        try {
          final progress = session.progress();
          if (_closed || _cancelRequested || progress == null) return;
          _progress = progress;
          _stage = progress.downloading
              ? LibraryModelInstallationStage.downloading
              : LibraryModelInstallationStage.preparing;
          notifyListeners();
        } on Object catch (error) {
          _poll?.cancel();
          _error ??= error;
          _requestNativeCancellation();
        }
      });
      await session.prepare();
      _poll?.cancel();
      if (_error != null) {
        throw StateError("Model installation progress failed");
      }
      if (_closed || _cancelRequested) return;
      _stage = LibraryModelInstallationStage.enabling;
      notifyListeners();
      await enable(session.commit);
      _stage = LibraryModelInstallationStage.enabled;
    } on Object catch (error) {
      _stage = _error == null && (_cancelRequested || _closed)
          ? LibraryModelInstallationStage.cancelled
          : LibraryModelInstallationStage.failed;
      if (_stage == LibraryModelInstallationStage.failed) _error ??= error;
    } finally {
      _poll?.cancel();
      _poll = null;
      final session = _session;
      _session = null;
      try {
        session?.dispose();
      } on Object catch (error) {
        _error ??= error;
        _stage = LibraryModelInstallationStage.failed;
      } finally {
        _operation = null;
        _finishedAt = DateTime.now();
        if (_stage != LibraryModelInstallationStage.enabled &&
            _error == null &&
            (_closed || _cancelRequested)) {
          _stage = LibraryModelInstallationStage.cancelled;
        }
        if (!_closed) notifyListeners();
      }
    }
  }

  void cancel() {
    if (!canCancel) return;
    _cancelRequested = true;
    _stage = LibraryModelInstallationStage.stopping;
    _requestNativeCancellation();
    notifyListeners();
  }

  void _requestNativeCancellation() {
    try {
      _session?.cancel();
    } on Object catch (error) {
      _error ??= error;
    }
  }

  void dismiss() {
    if (_closed || busy) return;
    _stage = LibraryModelInstallationStage.idle;
    _error = null;
    notifyListeners();
  }

  Future<void> close() async {
    _closed = true;
    _cancelRequested = true;
    try {
      _session?.cancel();
    } finally {
      await _operation;
    }
  }
}
