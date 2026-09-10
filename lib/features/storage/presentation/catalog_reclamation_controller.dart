import "dart:async";

import "package:flutter/foundation.dart";

import "../application/storage_settings.dart";
import "../domain/storage_models.dart";
import "storage_configuration_projection.dart";
import "storage_snapshot_coordinator.dart";

enum CatalogReclamationAction { none, starting, cancelling }

enum CatalogReclamationRetryTarget { status, storageSnapshot }

@immutable
class CatalogReclamationFailure {
  const CatalogReclamationFailure(this.message, this.retryTarget);

  final String message;
  final CatalogReclamationRetryTarget retryTarget;
}

@immutable
class CatalogReclamationViewState {
  const CatalogReclamationViewState({
    this.reclamation,
    this.storageStatus,
    this.action = CatalogReclamationAction.none,
    this.failure,
  });

  final CatalogReclamationModel? reclamation;
  final StorageStatusModel? storageStatus;
  final CatalogReclamationAction action;
  final CatalogReclamationFailure? failure;

  String? get errorMessage => failure?.message;

  bool get isActionPending => action != CatalogReclamationAction.none;
}

/// Orders storage snapshots and status polls within a single command epoch.
class CatalogReclamationSnapshotRequest {
  const CatalogReclamationSnapshotRequest._(
    this._owner,
    this._epoch,
    this._sequence,
    this._configurationRevision,
  );

  final CatalogReclamationController _owner;
  final int _epoch;
  final int _sequence;
  final int _configurationRevision;
}

class StorageConfigurationSaveRequest {
  const StorageConfigurationSaveRequest._(this._snapshot);

  final CatalogReclamationSnapshotRequest _snapshot;
}

class CatalogReclamationController extends ChangeNotifier
    implements StorageSnapshotReader {
  CatalogReclamationController({
    required this._gateway,
    this._onStorageRefreshed,
  });

  final StorageSettingsGateway _gateway;
  final ValueChanged<StorageStatusModel>? _onStorageRefreshed;
  late final StorageSnapshotCoordinator _storageSnapshots =
      StorageSnapshotCoordinator(this);
  CatalogReclamationViewState _state = const CatalogReclamationViewState();
  Timer? _poller;
  int _epoch = 0;
  int _nextSequence = 0;
  int _lastAppliedSequence = 0;
  CatalogReclamationSnapshotRequest? _activeRefresh;
  bool _isDisposed = false;
  String? _lastRefreshedTerminalOperationId;
  StorageConfigurationProjection? _confirmedConfiguration;
  int _configurationRevision = 0;
  StorageConfigurationSaveRequest? _configurationSave;

  CatalogReclamationViewState get state => _state;

  CatalogReclamationFailure? get _unresolvedStorageFailure =>
      _state.failure?.retryTarget ==
          CatalogReclamationRetryTarget.storageSnapshot
      ? _state.failure
      : null;

  CatalogReclamationSnapshotRequest beginStorageSnapshot() =>
      CatalogReclamationSnapshotRequest._(
        this,
        _epoch,
        ++_nextSequence,
        _configurationRevision,
      );

  StorageConfigurationSaveRequest beginConfigurationSave() =>
      _configurationSave = StorageConfigurationSaveRequest._(
        beginStorageSnapshot(),
      );

  Future<bool> completeConfigurationSave(
    StorageConfigurationSaveRequest request,
    StorageStatusModel status,
  ) async {
    if (_isDisposed || !identical(_configurationSave, request)) {
      return false;
    }
    _configurationSave = null;
    final confirmsUsage = _ownsRead(request._snapshot);
    if (confirmsUsage) {
      _lastAppliedSequence = request._snapshot._sequence;
    }
    final configuration = StorageConfigurationProjection.fromStatus(status);
    _confirmedConfiguration = configuration;
    _configurationRevision += 1;
    final usage = confirmsUsage ? status : _state.storageStatus ?? status;
    _publish(
      storageStatus: configuration.applyTo(usage),
      reclamation: confirmsUsage
          ? status.catalogReclamation
          : _state.reclamation,
      failure: _state.failure,
    );
    await _storageSnapshots.refresh();
    return true;
  }

  bool acceptStorageSnapshot(
    CatalogReclamationSnapshotRequest request,
    StorageStatusModel status,
  ) {
    if (!_ownsRead(request)) {
      return false;
    }
    _lastAppliedSequence = request._sequence;
    final reclamation = status.catalogReclamation;
    if (!reclamation.isActive) {
      _lastRefreshedTerminalOperationId = reclamation.operationId;
    }
    final configuration = _confirmedConfiguration;
    final projected =
        request._configurationRevision < _configurationRevision &&
            configuration != null
        ? configuration.applyTo(status)
        : status;
    _publish(reclamation: reclamation, storageStatus: projected);
    return true;
  }

  void failStorageSnapshot(
    CatalogReclamationSnapshotRequest request,
    Object error,
  ) {
    _failRead(request, error, CatalogReclamationRetryTarget.storageSnapshot);
  }

  void _failRead(
    CatalogReclamationSnapshotRequest request,
    Object error,
    CatalogReclamationRetryTarget retryTarget,
  ) {
    if (_ownsRead(request)) {
      _lastAppliedSequence = request._sequence;
      _publish(
        failure: CatalogReclamationFailure(
          _errorText(error),
          _unresolvedStorageFailure?.retryTarget ?? retryTarget,
        ),
      );
    }
  }

  Future<void> retry() =>
      _state.failure?.retryTarget ==
          CatalogReclamationRetryTarget.storageSnapshot
      ? _storageSnapshots.refresh()
      : refresh();

  Future<void> refresh() async {
    if (_isDisposed ||
        _activeRefresh != null ||
        _storageSnapshots.isBusy ||
        _state.isActionPending) {
      return;
    }
    final request = beginStorageSnapshot();
    _activeRefresh = request;
    try {
      final reclamation = await _gateway.loadCatalogReclamation();
      if (!_ownsRead(request)) {
        return;
      }
      _lastAppliedSequence = request._sequence;
      _publish(reclamation: reclamation, failure: _unresolvedStorageFailure);
      await _refreshStorageAfterTerminal(reclamation);
    } on Object catch (error) {
      _failRead(request, error, CatalogReclamationRetryTarget.status);
    } finally {
      _releaseRefresh(request);
    }
  }

  Future<void> start() async {
    final reclamation = _state.reclamation;
    if (_isDisposed ||
        _state.isActionPending ||
        reclamation == null ||
        !reclamation.canRetry) {
      return;
    }
    final epoch = ++_epoch;
    _publish(
      action: CatalogReclamationAction.starting,
      failure: _unresolvedStorageFailure,
    );
    try {
      final next = await _gateway.startCatalogReclamation(
        operationId:
            "catalog-reclamation-${DateTime.now().microsecondsSinceEpoch}",
      );
      if (_finishAction(epoch, reclamation: next)) {
        await _refreshStorageAfterTerminal(next);
      }
    } on Object catch (error) {
      _finishAction(epoch, errorMessage: _errorText(error));
    }
  }

  Future<void> cancel() async {
    final reclamation = _state.reclamation;
    final operationId = reclamation?.operationId;
    if (_isDisposed ||
        _state.isActionPending ||
        reclamation == null ||
        !reclamation.isActive ||
        operationId == null) {
      return;
    }
    final epoch = ++_epoch;
    _publish(
      action: CatalogReclamationAction.cancelling,
      failure: _unresolvedStorageFailure,
    );
    try {
      final accepted = await _gateway.cancelCatalogReclamation(
        operationId: operationId,
      );
      _finishAction(
        epoch,
        errorMessage: accepted ? null : "图库数据整理任务已经结束，无法再取消",
      );
    } on Object catch (error) {
      _finishAction(epoch, errorMessage: _errorText(error));
    }
  }

  Future<void> _refreshStorageAfterTerminal(
    CatalogReclamationModel reclamation,
  ) async {
    final operationId = reclamation.operationId;
    if (_isDisposed ||
        _state.isActionPending ||
        reclamation.isActive ||
        operationId == null ||
        operationId == _lastRefreshedTerminalOperationId) {
      return;
    }
    await _storageSnapshots.refresh();
  }

  @override
  StorageSnapshotReadContext get storageSnapshotReadContext =>
      StorageSnapshotReadContext(
        StorageSnapshotVersion(_epoch, _configurationRevision),
        _isDisposed || _state.isActionPending
            ? StorageSnapshotReadAvailability.suspended
            : StorageSnapshotReadAvailability.available,
      );

  @override
  Future<StorageSnapshotReadOutcome> readStorageSnapshot(
    StorageSnapshotVersion version,
  ) async {
    final context = storageSnapshotReadContext;
    if (context.availability == StorageSnapshotReadAvailability.suspended ||
        context.version != version) {
      return StorageSnapshotReadOutcome.superseded;
    }
    final request = beginStorageSnapshot();
    try {
      final status = await _gateway.load();
      if (acceptStorageSnapshot(request, status) && !_isDisposed) {
        _onStorageRefreshed?.call(_state.storageStatus!);
        return StorageSnapshotReadOutcome.settled;
      }
    } on Object catch (error) {
      if (_ownsRead(request)) {
        failStorageSnapshot(request, error);
        return StorageSnapshotReadOutcome.settled;
      }
    }
    return StorageSnapshotReadOutcome.superseded;
  }

  void _releaseRefresh(CatalogReclamationSnapshotRequest request) {
    if (identical(_activeRefresh, request)) {
      _activeRefresh = null;
    }
  }

  bool _finishAction(
    int epoch, {
    CatalogReclamationModel? reclamation,
    String? errorMessage,
  }) {
    if (_isDisposed || epoch != _epoch) {
      return false;
    }
    // Reads begun while a command was pending cannot describe its result.
    _epoch += 1;
    _publish(
      reclamation: reclamation,
      action: CatalogReclamationAction.none,
      failure: errorMessage == null
          ? _unresolvedStorageFailure
          : CatalogReclamationFailure(
              errorMessage,
              _unresolvedStorageFailure?.retryTarget ??
                  CatalogReclamationRetryTarget.status,
            ),
    );
    _storageSnapshots.wake();
    return true;
  }

  bool _ownsRead(CatalogReclamationSnapshotRequest request) =>
      !_isDisposed &&
      !_state.isActionPending &&
      identical(request._owner, this) &&
      request._epoch == _epoch &&
      request._sequence > _lastAppliedSequence;

  void _publish({
    CatalogReclamationModel? reclamation,
    StorageStatusModel? storageStatus,
    CatalogReclamationAction? action,
    CatalogReclamationFailure? failure,
  }) {
    _state = CatalogReclamationViewState(
      reclamation: reclamation ?? _state.reclamation,
      storageStatus: storageStatus ?? _state.storageStatus,
      action: action ?? _state.action,
      failure: failure,
    );
    if (_state.reclamation?.isActive ?? false) {
      _poller ??= Timer.periodic(
        const Duration(milliseconds: 500),
        (_) => unawaited(refresh()),
      );
    } else {
      _poller?.cancel();
      _poller = null;
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _isDisposed = true;
    _epoch += 1;
    _poller?.cancel();
    _storageSnapshots.dispose();
    super.dispose();
  }
}

String _errorText(Object error) => switch (error) {
  StorageSettingsFailure(:final message) => message,
  _ => error.toString(),
};
