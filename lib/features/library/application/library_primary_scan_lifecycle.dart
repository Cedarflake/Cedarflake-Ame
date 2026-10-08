import "dart:async";

import "../adapters/directory_picker.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_scan_control.dart";
import "library_scan_execution.dart";
import "library_scan_restoration.dart";
import "library_scan_run.dart";
import "library_scan_session.dart";
import "library_scan_shutdown.dart";
import "library_scan_source_admission.dart";
import "library_scan_target.dart";
import "library_scanner.dart";

abstract interface class LibraryPrimaryScanHost {
  LibraryState get libraryState;
  void publishPrimaryScan(LibraryPrimaryScanSnapshot snapshot);
  void preparePrimaryScanExecution();
  Future<bool> reloadPrimaryScanCatalog();
}

/// Owns one primary task, including retained intent when no stream is running.
class LibraryPrimaryScanLifecycle implements LibraryScanRunListener {
  LibraryPrimaryScanLifecycle({
    required LibraryPrimaryScanSnapshot initial,
    required this._host,
    required this._scanner,
    required this._picker,
    required this._catalog,
    required this._admission,
    required LibraryScanShutdownCoordinator shutdown,
    required this._previewEdge,
  }) : _state = initial,
       _shutdown = shutdown {
    shutdown.attach(this, suspendForShutdown);
  }

  LibraryPrimaryScanSnapshot _state;
  final LibraryPrimaryScanHost _host;
  final LibraryScanner _scanner;
  final DirectoryPicker _picker;
  final LibraryCatalog _catalog;
  final LibraryScanExecutionCoordinator _admission;
  final LibraryScanShutdownCoordinator _shutdown;
  final int _previewEdge;
  final LibraryScanSession _session = LibraryScanSession();
  LibraryScanRun? _run;
  Future<void> _commands = Future<void>.value();
  Future<void>? _retainedCancellation;
  int _sequence = 0;
  int _restorationSequence = 0;
  int _runGeneration = 0;
  bool _isDisposed = false;
  bool _isClosing = false;

  LibraryPrimaryScanSnapshot get snapshot => _state;
  bool get _isUnavailable =>
      _isDisposed || _isClosing || _shutdown.isShuttingDown;

  void _publish(LibraryPrimaryScanSnapshot next) {
    if (_isDisposed) {
      return;
    }
    _state = next;
    _host.publishPrimaryScan(next);
  }

  Future<void> restore() {
    return LibraryScanRestoration(
      loadRecoverable: _scanner.loadRecoverableScan,
      loadPaused: _scanner.loadPausedScan,
      session: _session,
      readState: () => _state,
      writeState: _publish,
      readScanSequence: () => _restorationSequence,
      isUnavailable: () => _isUnavailable,
    ).restore();
  }

  void invalidateRestoration() => _restorationSequence += 1;

  void rootUnregistered(LibraryRoot root) {
    invalidateRestoration();
    if (_state.rootPath != root.path &&
        _state.displayRootPath != root.displayPath) {
      return;
    }
    _sequence += 1;
    _session.clear();
    _publish(_idleSnapshot());
  }

  LibraryPrimaryScanSnapshot _idleSnapshot() => LibraryPrimaryScanSnapshot(
    status: _host.libraryState.roots.isEmpty
        ? LibraryStatus.empty
        : LibraryStatus.completed,
  );

  Future<void> chooseDirectoryAndScan() =>
      _chooseDirectory(const LibraryDirectoryScan());

  Future<void> chooseDirectoryAndRelocate(LibraryRoot root) =>
      _chooseDirectory(LibraryRootRelocation(root));

  Future<void> _chooseDirectory(LibraryScanTarget target) async {
    final retiring = _run;
    if (retiring != null && retiring.didReceiveTerminal) {
      await retiring.retired;
    }
    if (_isUnavailable ||
        _state.hasRetainedScan ||
        _host.libraryState.isBusy ||
        !_admission.tryAcquirePrimary(this)) {
      return;
    }
    invalidateRestoration();
    final sequence = ++_sequence;
    _publish(
      LibraryPrimaryScanSnapshot(
        status: LibraryStatus.choosingDirectory,
        taskKind: target.taskKind ?? LibraryTaskKind.import,
      ),
    );
    try {
      final directory = await _picker.pickDirectory();
      if (_isUnavailable || sequence != _sequence) {
        return;
      }
      if (directory == null) {
        _publish(_idleSnapshot());
      } else {
        await _scanDirectory(directory, target);
      }
    } on Object catch (error) {
      if (!_isUnavailable && sequence == _sequence) {
        _publish(_session.fail(_state, error));
      }
    } finally {
      if (_run == null) {
        _admission.releasePrimary(this);
      }
    }
  }

  Future<void> scanDirectory(String rootPath) =>
      _scanDirectory(rootPath, const LibraryDirectoryScan());

  Future<void> _scanDirectory(String rootPath, LibraryScanTarget target) =>
      _enqueue(
        allowsPaused: false,
        start: () {
          final scanId =
              "ame-${DateTime.now().microsecondsSinceEpoch}-${++_sequence}";
          return _start(
            RecoverableLibraryScan(
              scanId: scanId,
              rootPath: rootPath,
              displayRootPath: rootPath,
              itemLimit: null,
              entryLimit: null,
              previewEdge: _previewEdge,
              visitedEntries: 0,
              acceptedItems: 0,
              issueCount: 0,
            ),
            isResuming: false,
            target: target,
          );
        },
      );

  Future<void> _enqueue({
    required bool allowsPaused,
    required Future<void> Function() start,
  }) {
    final previous = _commands;
    final completion = Completer<void>();
    _commands = completion.future;
    return () async {
      await previous;
      try {
        final run = _run;
        if (run != null && run.didReceiveTerminal) {
          await run.retired;
        }
        final library = _host.libraryState;
        if (_isUnavailable ||
            _run != null ||
            _state.isScanning ||
            _state.status == LibraryStatus.discarding ||
            (_state.hasRetainedScan && !allowsPaused) ||
            library.isCommittedRemovalReloadPending ||
            library.status == LibraryStatus.removing) {
          return;
        }
        invalidateRestoration();
        await start();
      } finally {
        completion.complete();
      }
    }();
  }

  Future<void> _start(
    RecoverableLibraryScan checkpoint, {
    required bool isResuming,
    LibraryScanTarget target = const LibraryDirectoryScan(),
  }) async {
    final sourceAdmission = LibraryScanSourceAdmission(
      scan: checkpoint,
      roots: _host.libraryState.roots,
      target: target,
      isResuming: isResuming,
    );
    if (!_admission.tryAcquirePrimary(this)) {
      const message = "其他图库更新正在运行，请等待更新结束或先取消更新，然后继续此任务。";
      _publish(
        isResuming
            ? _state.copyWith(errorMessage: message)
            : LibraryPrimaryScanSnapshot(
                status: LibraryStatus.failed,
                scanId: checkpoint.scanId,
                rootPath: checkpoint.rootPath,
                displayRootPath: checkpoint.displayRootPath,
                taskKind: sourceAdmission.initialTaskKind,
                errorMessage: message,
              ),
      );
      return;
    }
    _host.preparePrimaryScanExecution();
    final run = LibraryScanRun(
      scanId: checkpoint.scanId,
      generation: ++_runGeneration,
      sourceAdmission: sourceAdmission,
      scanner: _scanner,
    );
    _run = run;
    try {
      final kind =
          target.taskKind ??
          (_state.scanId == checkpoint.scanId && _state.taskKind != null
              ? _state.taskKind!
              : sourceAdmission.initialTaskKind);
      _session.begin(checkpoint, target: target);
      _publish(
        LibraryPrimaryScanSnapshot(
          status: LibraryStatus.scanning,
          scanId: checkpoint.scanId,
          rootPath: checkpoint.rootPath,
          displayRootPath: checkpoint.displayRootPath,
          taskKind: kind,
          visitedEntries: checkpoint.visitedEntries,
          stagedAssetCount: checkpoint.acceptedItems,
          issueCount: checkpoint.issueCount,
          itemLimit: checkpoint.itemLimit,
          entryLimit: checkpoint.entryLimit,
          isResumingScan: isResuming,
        ),
      );
      final stream = isResuming
          ? _scanner.resume(
              scanId: checkpoint.scanId,
              rootPath: checkpoint.rootPath,
              itemLimit: checkpoint.itemLimit,
              entryLimit: checkpoint.entryLimit,
              previewEdge: checkpoint.previewEdge,
            )
          : target.open(_scanner, checkpoint);
      run.listen(stream, this);
    } on Object catch (error) {
      _release(run);
      _publish(_session.fail(_state, error));
      rethrow;
    }
  }

  void pause() {
    final run = _run;
    if (!_isUnavailable &&
        run != null &&
        _state.status == LibraryStatus.scanning &&
        run.control.request(LibraryScanCommand.pause)) {
      _publish(
        _state.copyWith(
          status: LibraryStatus.pausing,
          errorMessage: run.control.failure?.toString(),
        ),
      );
    }
  }

  Future<void> cancel() {
    if (_state.hasRetainedScan) {
      return _retainedCancellation ??= _cancelRetained();
    }
    final run = _run;
    if (!_isUnavailable &&
        run != null &&
        _state.isScanning &&
        run.control.request(LibraryScanCommand.cancel)) {
      _publish(
        _state.copyWith(
          status: LibraryStatus.cancelling,
          errorMessage: run.control.failure?.toString(),
        ),
      );
    }
    return Future<void>.value();
  }

  Future<void> _cancelRetained() async {
    final scanId = _state.scanId;
    if (_isUnavailable ||
        scanId == null ||
        _state.status != LibraryStatus.paused) {
      return;
    }
    invalidateRestoration();
    final sequence = ++_sequence;
    _publish(
      _state.copyWith(status: LibraryStatus.discarding, errorMessage: null),
    );
    try {
      final run = _run;
      if (run != null) {
        await run.streamDone;
      }
      await _scanner.cancelRetainedScan(scanId);
      if (!_isDisposed && sequence == _sequence && _state.scanId == scanId) {
        _session.clear();
        _publish(_idleSnapshot());
      }
    } on Object catch (error) {
      if (!_isDisposed && sequence == _sequence && _state.scanId == scanId) {
        _publish(
          _state.copyWith(
            status: LibraryStatus.paused,
            errorMessage: error.toString(),
          ),
        );
      }
    } finally {
      _retainedCancellation = null;
    }
  }

  Future<void> resume() async {
    final checkpoint = _session.pausedScan;
    if (checkpoint != null && _state.status == LibraryStatus.paused) {
      await _resume(checkpoint);
    }
  }

  Future<void> _resume(RecoverableLibraryScan checkpoint) => _enqueue(
    allowsPaused: true,
    start: () {
      _sequence += 1;
      return _start(checkpoint, isResuming: true);
    },
  );

  Future<void> retry() async {
    final retiring = _run;
    if (retiring != null && retiring.didReceiveTerminal) {
      await retiring.retired;
    }
    if (_isUnavailable || _state.blocksExecution || _state.hasRetainedScan) {
      return;
    }
    if (_state.publication == LibraryScanPublication.reloadPending) {
      final sequence = ++_sequence;
      _publish(
        _state.copyWith(status: LibraryStatus.refreshing, errorMessage: null),
      );
      await _reloadPublishedCatalog(sequence);
      return;
    }
    if (_state.rootPath == null) {
      invalidateRestoration();
      _publish(_idleSnapshot());
      await restore();
      return;
    }
    invalidateRestoration();
    final sequence = ++_sequence;
    final requested = _state;
    try {
      final recoverable = await _scanner.loadRecoverableScan();
      final paused = recoverable ?? await _scanner.loadPausedScan();
      if (_isUnavailable || sequence != _sequence) {
        return;
      }
      if (paused != null &&
          (paused.scanId == requested.scanId ||
              paused.rootPath == requested.rootPath)) {
        await _resume(paused);
      } else {
        await _scanDirectory(requested.rootPath!, _session.target);
      }
    } on Object catch (error) {
      if (!_isUnavailable && sequence == _sequence) {
        _publish(_session.fail(_state, error));
      }
    }
  }

  void dismiss() {
    if (_state.status != LibraryStatus.completed &&
        _state.status != LibraryStatus.failed &&
        _state.status != LibraryStatus.cancelled) {
      return;
    }
    _sequence += 1;
    invalidateRestoration();
    _session.clear();
    _publish(_idleSnapshot());
  }

  @override
  void onScanUpdate(LibraryScanRun run, LibraryScanUpdate update) {
    if (!_owns(run)) {
      return;
    }
    if (update is LibraryScanStarted && run.sourceAdmission.needsRefresh) {
      unawaited(_refreshAdmittedSource(run));
    }
    final transition = _session.apply(_state, update);
    final controlStatus = switch (run.control.command) {
      LibraryScanCommand.cancel => LibraryStatus.cancelling,
      LibraryScanCommand.pause => LibraryStatus.pausing,
      LibraryScanCommand.suspend || null => null,
    };
    _publish(
      controlStatus == null
          ? transition.state
          : transition.state.copyWith(
              status: controlStatus,
              errorMessage: run.control.failure?.toString(),
            ),
    );
    if (transition.shouldReloadCatalog) {
      unawaited(_reloadPublishedCatalog(_sequence));
    }
  }

  @override
  void onScanError(LibraryScanRun run, Object error) {
    if (_owns(run)) {
      _publish(
        _session.fail(_state, error).copyWith(status: LibraryStatus.cancelling),
      );
    }
  }

  Future<void> _refreshAdmittedSource(LibraryScanRun run) async {
    final sequence = _sequence;
    try {
      final applied = await run.sourceAdmission.refresh(
        _host.reloadPrimaryScanCatalog,
      );
      if (!_owns(run) || sequence != _sequence || _state.scanId != run.scanId) {
        return;
      }
      if (!applied) {
        throw const LibraryCatalogFailure(
          code: "catalog_source_view_superseded",
          message: "无法刷新图库位置信息，请重试。",
        );
      }
      if (run.sourceAdmission.hasAdmittedExistingRoot(
        _host.libraryState.roots,
        displayRootPath: _state.displayRootPath,
      )) {
        _session.confirmSourceAdmission();
        _publish(_state.copyWith(taskKind: LibraryTaskKind.update));
      }
    } on Object catch (error) {
      if (_owns(run) && sequence == _sequence && _state.scanId == run.scanId) {
        _publish(_state.copyWith(errorMessage: error.toString()));
      }
    }
  }

  @override
  void onScanDone(LibraryScanRun run) {
    if (!_owns(run)) {
      return;
    }
    if (run.didReceiveTerminal) {
      if (run.hasProtocolFailure) {
        _publish(_state.copyWith(status: LibraryStatus.failed));
      }
      if (run.sourceAdmission.needsRefresh) {
        unawaited(_releaseAfterSourceRefresh(run));
        return;
      }
      _release(run);
    } else {
      unawaited(_reconcileEndedScan(run, _sequence));
    }
  }

  Future<void> _releaseAfterSourceRefresh(LibraryScanRun run) async {
    await _refreshAdmittedSource(run);
    _release(run);
  }

  bool _owns(LibraryScanRun run) =>
      !_isDisposed &&
      identical(_run, run) &&
      _run?.generation == run.generation;

  void _release(LibraryScanRun run) {
    if (identical(_run, run)) {
      _run = null;
      _admission.releasePrimary(this);
    }
    unawaited(run.dispose());
  }

  Future<void> _reconcileEndedScan(LibraryScanRun run, int sequence) async {
    try {
      final catalog = await _catalog.load(
        maxItems: libraryCatalogWindow,
        query: _host.libraryState.query,
      );
      if (!_owns(run) || sequence != _sequence) {
        return;
      }
      final published = catalog.roots.where(
        (root) => root.activeScanId == run.scanId,
      );
      if (run.didStart && published.isNotEmpty) {
        final root = published.first;
        final wasLimited =
            (_state.itemLimit != null &&
                root.assetCount >= _state.itemLimit!) ||
            (_state.entryLimit != null &&
                _state.visitedEntries >= _state.entryLimit!);
        _publish(
          _session
              .apply(
                _state,
                LibraryScanCompleted(
                  assetCount: root.assetCount,
                  issueCount: root.issueCount,
                  catalogPath: catalog.catalogPath,
                  wasLimited: wasLimited,
                ),
              )
              .state,
        );
        await _reloadPublishedCatalog(sequence);
        return;
      }
      final paused = await _scanner.loadPausedScan();
      if (!_owns(run) || sequence != _sequence) {
        return;
      }
      if (paused != null && paused.scanId == run.scanId) {
        _session.restorePaused(paused);
        _publish(
          _state.copyWith(
            status: LibraryStatus.paused,
            visitedEntries: paused.visitedEntries,
            stagedAssetCount: paused.acceptedItems,
            issueCount: paused.issueCount,
            isResumingScan: false,
          ),
        );
      } else if (_state.status == LibraryStatus.cancelling) {
        _publish(
          _session
              .apply(
                _state,
                LibraryScanCancelled(
                  acceptedItems: _state.stagedAssetCount,
                  issueCount: _state.issueCount,
                ),
              )
              .state,
        );
      } else {
        _publish(_session.finish(_state));
      }
    } on Object catch (error) {
      if (_owns(run) && sequence == _sequence) {
        _publish(
          _session.fail(
            _state,
            LibraryScanFailure(
              code: "scan_terminal_reconciliation_failed",
              message: error.toString(),
            ),
          ),
        );
      }
    } finally {
      if (_owns(run)) {
        _release(run);
      }
    }
  }

  Future<void> _reloadPublishedCatalog(int sequence) async {
    try {
      final applied = await _host.reloadPrimaryScanCatalog();
      if (!applied) {
        throw const LibraryCatalogFailure(
          code: "catalog_publication_view_superseded",
          message: "图库已保存，但当前显示刷新已被其他操作替换，请重试刷新显示。",
        );
      }
      if (!_isDisposed &&
          sequence == _sequence &&
          _state.status == LibraryStatus.refreshing) {
        _publish(
          _state.copyWith(
            status: LibraryStatus.completed,
            publication: LibraryScanPublication.visible,
            errorMessage: null,
          ),
        );
      }
    } on Object catch (error) {
      if (!_isDisposed && sequence == _sequence) {
        _publish(_session.fail(_state, error));
      }
    }
  }

  Future<void> suspendForShutdown() async {
    if (_isDisposed || _isClosing) {
      return;
    }
    _isClosing = true;
    await _commands;
    await _retainedCancellation;
    final run = _run;
    if (!_isDisposed && run != null && !run.isDone) {
      run.control.request(LibraryScanCommand.suspend);
      await run.streamDone;
    }
  }

  void dispose() {
    _isDisposed = true;
    _sequence += 1;
    invalidateRestoration();
    _shutdown.detach(this);
    final run = _run;
    _run = null;
    _admission.releasePrimary(this);
    if (run != null) {
      if (!_isClosing && !_shutdown.isShuttingDown) {
        run.control.request(LibraryScanCommand.cancel);
      }
      unawaited(run.dispose());
    }
  }
}
