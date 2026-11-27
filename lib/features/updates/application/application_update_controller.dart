import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/application_update.dart";
import "application_update_state.dart";
import "application_update_environment.dart";
import "ports/application_update_service.dart";

final applicationUpdateControllerProvider =
    Provider<ApplicationUpdateController>(
      (ref) => throw StateError(
        "ApplicationUpdateController must be bound during application startup",
      ),
    );

class ApplicationUpdateController extends ChangeNotifier {
  ApplicationUpdateController({
    required this._service,
    required this._preferences,
    required this._restarter,
    required ApplicationUpdateChannel defaultChannel,
  }) : _state = ApplicationUpdateState(
         preferences: ApplicationUpdatePreferences(channel: defaultChannel),
       );

  final ApplicationUpdateService _service;
  final ApplicationUpdatePreferenceStore _preferences;
  final ApplicationUpdateRestarter _restarter;
  ApplicationUpdateState _state;
  ApplicationUpdateState get state => _state;

  ApplicationUpdateDownload? _download;
  Future<void>? _initialization;
  Future<void>? _checking;
  Future<void>? _transfer;
  Future<void>? _storeOpening;
  Timer? _automaticCheck;
  Timer? _progressTimer;
  int _generation = 0;
  bool _closed = false;
  bool _cancelRequested = false;
  ApplicationUpdateResult? lastResult;

  Future<void> initialize() => _initialization ??= _initialize();

  Future<void> _initialize() async {
    try {
      final environment = await ApplicationUpdateEnvironment.load(
        service: _service,
        preferences: _preferences,
        defaultChannel: _state.preferences.channel,
      );
      lastResult = environment.result;
      if (_closed) return;
      _publish(
        ApplicationUpdateState(
          preferences: environment.preferences,
          phase: ApplicationUpdatePhase.idle,
          delivery: environment.delivery,
        ),
      );
      _scheduleAutomaticCheck(const Duration(milliseconds: 500));
    } on ApplicationUpdateFailure catch (failure) {
      _initialization = null;
      if (!_closed) _setPhase(ApplicationUpdatePhase.failed, failure: failure);
    }
  }

  Future<void> setPreferences(ApplicationUpdatePreferences preferences) async {
    await initialize();
    if (_closed ||
        !_state.supported ||
        _state.phase == ApplicationUpdatePhase.restarting) {
      return;
    }
    await _preferences.save(preferences);
    if (_closed) return;
    final channelChanged = preferences.channel != _state.preferences.channel;
    if (channelChanged) {
      _generation++;
      final previous = _download;
      _download = null;
      previous?.cancel();
      _publish(
        ApplicationUpdateState(
          preferences: preferences,
          phase: ApplicationUpdatePhase.idle,
          delivery: _state.delivery,
        ),
      );
      await _transfer;
      await previous?.close();
    } else {
      _publish(
        ApplicationUpdateState(
          preferences: preferences,
          phase: _state.phase,
          delivery: _state.delivery,
          release: _state.release,
          progress: _state.progress,
          failure: _state.failure,
        ),
      );
    }
    _scheduleAutomaticCheck(const Duration(milliseconds: 500));
  }

  Future<void> check() async {
    final running = _checking;
    if (running != null) {
      await running;
      if (!_closed && _state.phase == ApplicationUpdatePhase.idle) {
        await check();
      }
      return;
    }
    final operation = _checkRelease();
    _checking = operation;
    await operation.whenComplete(() {
      if (identical(_checking, operation)) _checking = null;
    });
  }

  Future<void> openStore() {
    final pending = _storeOpening;
    if (pending != null) return pending;
    final operation = _openStore();
    _storeOpening = operation;
    return operation.whenComplete(() {
      if (identical(_storeOpening, operation)) _storeOpening = null;
    });
  }

  Future<void> _openStore() async {
    await initialize();
    if (_closed || !_state.storeManaged) return;
    try {
      await _service.openStore();
      if (!_closed) _setPhase(ApplicationUpdatePhase.idle);
    } on ApplicationUpdateFailure catch (failure) {
      if (!_closed) _setPhase(ApplicationUpdatePhase.failed, failure: failure);
    }
  }

  Future<void> _checkRelease() async {
    await initialize();
    if (_closed || !_state.supported || _state.busy || _transfer != null) {
      return;
    }
    if (_state.phase == ApplicationUpdatePhase.ready) return;
    final generation = ++_generation;
    final previous = _download;
    _download = null;
    _setPhase(ApplicationUpdatePhase.checking);
    await previous?.close();
    try {
      final download = await _service.check(_state.preferences.channel);
      if (!_admits(generation)) {
        await download?.close();
        return;
      }
      _download = download;
      _setPhase(
        download == null
            ? ApplicationUpdatePhase.current
            : ApplicationUpdatePhase.available,
      );
      if (download != null &&
          _state.preferences.policy == ApplicationUpdatePolicy.download) {
        await this.download();
      }
    } on ApplicationUpdateFailure catch (failure) {
      if (_admits(generation)) {
        _setPhase(ApplicationUpdatePhase.failed, failure: failure);
      }
    }
  }

  Future<void> download() {
    final active = _transfer;
    if (active != null) return active;
    final download = _download;
    if (_closed || download == null || _state.busy) return Future.value();
    if (_state.phase == ApplicationUpdatePhase.ready) return Future.value();
    final operation = _downloadRelease(download, _generation);
    _transfer = operation;
    return operation.whenComplete(() {
      if (identical(_transfer, operation)) _transfer = null;
    });
  }

  Future<void> _downloadRelease(
    ApplicationUpdateDownload download,
    int generation,
  ) async {
    _cancelRequested = false;
    _setPhase(ApplicationUpdatePhase.downloading);
    _progressTimer = Timer.periodic(const Duration(milliseconds: 200), (_) {
      if (!_admits(generation)) return;
      final progress = download.progress;
      if (progress.receivedBytes == _state.progress?.receivedBytes &&
          progress.totalBytes == _state.progress?.totalBytes &&
          progress.phase == _state.progress?.phase) {
        return;
      }
      final phase = switch ((_cancelRequested, progress.phase)) {
        (true, _) => ApplicationUpdatePhase.stopping,
        (false, ApplicationDownloadPhase.verifying) =>
          ApplicationUpdatePhase.verifying,
        _ => ApplicationUpdatePhase.downloading,
      };
      _setPhase(phase, progress: progress);
    });
    try {
      await download.download();
      if (!_admits(generation)) return;
      if (download.progress.phase == ApplicationDownloadPhase.cancelled) {
        _setPhase(ApplicationUpdatePhase.available);
        return;
      }
      if (_admits(generation)) {
        _setPhase(ApplicationUpdatePhase.ready, progress: download.progress);
      }
    } on ApplicationUpdateFailure catch (failure) {
      if (_admits(generation)) {
        _setPhase(ApplicationUpdatePhase.failed, failure: failure);
      }
    } finally {
      _progressTimer?.cancel();
      _progressTimer = null;
    }
  }

  void cancelDownload() {
    if (_state.phase != ApplicationUpdatePhase.downloading &&
        _state.phase != ApplicationUpdatePhase.verifying) {
      return;
    }
    _cancelRequested = true;
    _setPhase(ApplicationUpdatePhase.stopping, progress: _state.progress);
    _download?.cancel();
  }

  Future<void> restart() async {
    final download = _download;
    if (_closed ||
        download == null ||
        _state.phase != ApplicationUpdatePhase.ready) {
      return;
    }
    _setPhase(ApplicationUpdatePhase.restarting);
    try {
      await download.verify();
      if (_closed) return;
      _download = null;
      await _restarter.restart(download);
    } on ApplicationUpdateFailure catch (failure) {
      if (!_closed) {
        _download = download;
        _setPhase(
          failure.code == ApplicationUpdateFailureCode.damagedDownload
              ? ApplicationUpdatePhase.failed
              : ApplicationUpdatePhase.ready,
          failure: failure,
        );
      }
    } finally {
      if (_closed) await download.close();
    }
  }

  void _scheduleAutomaticCheck(Duration delay) {
    _automaticCheck?.cancel();
    if (_closed ||
        !_state.supported ||
        _state.preferences.policy == ApplicationUpdatePolicy.manual) {
      return;
    }
    _automaticCheck = Timer(delay, () {
      unawaited(
        check().whenComplete(
          () => _scheduleAutomaticCheck(const Duration(hours: 12)),
        ),
      );
    });
  }

  bool _admits(int generation) => !_closed && generation == _generation;

  void _setPhase(
    ApplicationUpdatePhase phase, {
    ApplicationDownloadProgress? progress,
    ApplicationUpdateFailure? failure,
  }) => _publish(
    ApplicationUpdateState(
      preferences: _state.preferences,
      delivery: _state.delivery,
      phase: phase,
      release: _download?.release,
      progress: progress,
      failure: failure,
    ),
  );

  void _publish(ApplicationUpdateState value) {
    if (_closed) return;
    _state = value;
    notifyListeners();
  }

  Future<void> close() async {
    if (_closed) return;
    _closed = true;
    _generation++;
    _automaticCheck?.cancel();
    _progressTimer?.cancel();
    _download?.cancel();
    await _initialization;
    await _checking;
    await _transfer;
    await _storeOpening;
    await _download?.close();
    _download = null;
  }
}
