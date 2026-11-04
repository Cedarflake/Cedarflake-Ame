import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/legacy.dart";

import "../domain/library_model_installation.dart";
import "ports/library_model_installation.dart";

typedef EnablePreparedLibraryModel =
    Future<void> Function(Future<void> Function() commit);

final libraryModelInstallationProvider =
    ChangeNotifierProvider<LibraryModelInstallation?>(
      (ref) =>
          throw StateError("Library model installation binding is required"),
    );

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
          _stage == LibraryModelInstallationStage.verifying ||
          _stage == LibraryModelInstallationStage.downloading);

  Future<void> start(EnablePreparedLibraryModel enable) {
    return _begin(enable, recovering: false);
  }

  Future<void> recover(EnablePreparedLibraryModel enable) {
    return _begin(enable, recovering: true);
  }

  Future<void> _begin(
    EnablePreparedLibraryModel enable, {
    required bool recovering,
  }) {
    if (_closed || _operation != null) {
      return Future.error(StateError("Model installation is unavailable"));
    }
    _cancelRequested = false;
    _progress = null;
    _error = null;
    _finishedAt = null;
    _sequence += 1;
    _stage = recovering
        ? LibraryModelInstallationStage.idle
        : LibraryModelInstallationStage.preparing;
    final operation = Future<void>.microtask(
      () => _run(enable, recovering: recovering),
    );
    _operation = operation;
    notifyListeners();
    return operation;
  }

  Future<void> _run(
    EnablePreparedLibraryModel enable, {
    required bool recovering,
  }) async {
    try {
      if (_closed || _cancelRequested) return;
      final session = recovering ? await _service.recover() : _service.create();
      _session = session;
      if (session == null) return;
      if (_closed || _cancelRequested) {
        session.cancel();
        return;
      }
      _stage = LibraryModelInstallationStage.preparing;
      notifyListeners();
      _poll = Timer.periodic(const Duration(milliseconds: 200), (_) {
        try {
          final progress = session.progress();
          if (_closed || _cancelRequested || progress == null) return;
          _progress = progress;
          _stage = progress.downloading
              ? LibraryModelInstallationStage.downloading
              : LibraryModelInstallationStage.verifying;
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
        if (_cancelRequested) await session?.discard();
      } on Object catch (error) {
        _error ??= error;
        _stage = LibraryModelInstallationStage.failed;
      }
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
    try {
      _session?.cancel();
    } finally {
      await _operation;
    }
  }
}
