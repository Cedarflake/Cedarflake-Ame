import "dart:async";
import "dart:collection";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_models.dart";
import "library_controller.dart";
import "library_scan_execution.dart";
import "library_scan_shutdown.dart";
import "library_scanner.dart";

const libraryUpdateMaxConcurrentRoots = 2;
const _previewEdge = 512;

enum LibraryRootUpdatePhase {
  queued,
  discovering,
  finalizing,
  refreshing,
  cancelling,
  completed,
  cancelled,
  stale,
  refreshFailed,
  failed,
}

class LibraryRootUpdateTask {
  const LibraryRootUpdateTask({
    required this.root,
    required this.scanId,
    this.phase = LibraryRootUpdatePhase.queued,
    this.visitedEntries = 0,
    this.acceptedItems = 0,
    this.issueCount = 0,
    this.validatedItems = 0,
    this.validationItemCount = 0,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final LibraryRoot root;
  final String scanId;
  final LibraryRootUpdatePhase phase;
  final int visitedEntries;
  final int acceptedItems;
  final int issueCount;
  final int validatedItems;
  final int validationItemCount;
  final String? errorMessage;

  bool get isActive => switch (phase) {
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing ||
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling => true,
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled ||
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => false,
  };

  bool get canCancel => switch (phase) {
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing => true,
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling ||
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled ||
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => false,
  };

  bool get canRetry => switch (phase) {
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => true,
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing ||
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling ||
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled => false,
  };

  LibraryRootUpdateTask copyWith({
    LibraryRootUpdatePhase? phase,
    int? visitedEntries,
    int? acceptedItems,
    int? issueCount,
    int? validatedItems,
    int? validationItemCount,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryRootUpdateTask(
      root: root,
      scanId: scanId,
      phase: phase ?? this.phase,
      visitedEntries: visitedEntries ?? this.visitedEntries,
      acceptedItems: acceptedItems ?? this.acceptedItems,
      issueCount: issueCount ?? this.issueCount,
      validatedItems: validatedItems ?? this.validatedItems,
      validationItemCount: validationItemCount ?? this.validationItemCount,
      errorMessage: errorMessage == _unchanged
          ? this.errorMessage
          : errorMessage as String?,
    );
  }
}

class LibraryUpdateState {
  const LibraryUpdateState({this.tasksByRootId = const {}});

  final Map<String, LibraryRootUpdateTask> tasksByRootId;

  List<LibraryRootUpdateTask> get tasks =>
      List.unmodifiable(tasksByRootId.values);

  Set<String> get activeRootIds => Set.unmodifiable({
    for (final task in tasksByRootId.values)
      if (task.isActive) task.root.id,
  });

  bool get hasActive => tasksByRootId.values.any((task) => task.isActive);

  bool get hasFeedback => tasksByRootId.isNotEmpty;

  LibraryUpdateState withTask(LibraryRootUpdateTask task) {
    return LibraryUpdateState(
      tasksByRootId: Map.unmodifiable({...tasksByRootId, task.root.id: task}),
    );
  }

  LibraryUpdateState withoutRoot(String rootId) {
    return LibraryUpdateState(
      tasksByRootId: Map.unmodifiable({...tasksByRootId}..remove(rootId)),
    );
  }

  LibraryUpdateState withoutTerminalTasks() {
    return LibraryUpdateState(
      tasksByRootId: Map.unmodifiable({
        for (final entry in tasksByRootId.entries)
          if (entry.value.isActive) entry.key: entry.value,
      }),
    );
  }
}

typedef LibraryUpdateCatalogRefresh = Future<void> Function();

final libraryUpdateCatalogRefreshProvider =
    Provider<LibraryUpdateCatalogRefresh>((ref) {
      return () async {
        final controller = ref.read(libraryControllerProvider.notifier);
        final didRefresh = await controller.refreshCurrentQuery();
        if (!didRefresh) {
          throw StateError("The updated catalog could not be displayed");
        }
      };
    });

final libraryUpdateConfiguredRootsProvider = Provider<List<LibraryRoot>>((ref) {
  return ref.watch(
    libraryControllerProvider.select((library) => library.roots),
  );
});

final libraryUpdatePrimaryBusyProvider = Provider<bool>((ref) {
  return ref.watch(
    libraryControllerProvider.select((library) => library.isBusy),
  );
});

final libraryUpdateRetainedRootProvider = Provider<String?>((ref) {
  return ref.watch(
    libraryControllerProvider.select((library) => library.retainedScanRootId),
  );
});

class _ActiveRootUpdate {
  _ActiveRootUpdate({required this.root, required this.scanId});

  final LibraryRoot root;
  final String scanId;
  final Completer<void> streamDone = Completer<void>();
  StreamSubscription<LibraryScanUpdate>? subscription;
  LibraryScanUpdate? terminalUpdate;
  LibraryRootUpdateTask? terminalTask;
  Object? streamError;
  bool cancelRequested = false;

  void cancelSubscription() {
    final cancellation = subscription?.cancel();
    subscription = null;
    if (cancellation != null) {
      unawaited(cancellation);
    }
  }
}

class LibraryUpdateController extends Notifier<LibraryUpdateState> {
  final Queue<String> _pendingRootIds = Queue();
  final Map<String, LibraryRoot> _pendingRoots = {};
  final Map<String, _ActiveRootUpdate> _activeRuns = {};
  final Set<String> _ownedRootIds = {};
  Future<void> _catalogRefreshQueue = Future<void>.value();
  LibraryScanShutdownCoordinator? _shutdownCoordinator;
  int _scanSequence = 0;
  bool _isDisposed = false;
  bool _isClosing = false;
  LibraryScanExecutionCoordinator? _scanExecutionCoordinator;

  @override
  LibraryUpdateState build() {
    final shutdownCoordinator = ref.read(
      libraryScanShutdownCoordinatorProvider,
    );
    final scanner = ref.read(libraryScannerProvider);
    final scanExecutionCoordinator = ref.read(
      libraryScanExecutionCoordinatorProvider,
    );
    _scanExecutionCoordinator = scanExecutionCoordinator;
    _shutdownCoordinator = shutdownCoordinator;
    shutdownCoordinator.attach(this, _cancelActiveUpdatesForShutdown);
    ref.onDispose(() {
      shutdownCoordinator.detach(this);
      _isDisposed = true;
      _pendingRootIds.clear();
      _pendingRoots.clear();
      for (final run in _activeRuns.values) {
        if (!_isClosing && !shutdownCoordinator.isShuttingDown) {
          scanner.cancel(run.scanId);
        }
        run.cancelSubscription();
        if (!run.streamDone.isCompleted) {
          run.streamDone.complete();
        }
      }
      _activeRuns.clear();
      for (final rootId in _ownedRootIds) {
        scanExecutionCoordinator.releaseUpdate(rootId);
      }
      _ownedRootIds.clear();
    });
    return const LibraryUpdateState();
  }

  void startUpdates(Iterable<String> rootIds) {
    if (_isDisposed ||
        _isClosing ||
        (_shutdownCoordinator?.isShuttingDown ?? false) ||
        ref.read(libraryUpdatePrimaryBusyProvider)) {
      return;
    }
    final scanExecutionCoordinator = _readScanExecutionCoordinator();
    if (!scanExecutionCoordinator.canStartUpdates) {
      return;
    }
    final configuredRoots = {
      for (final root in ref.read(libraryUpdateConfiguredRootsProvider))
        root.id: root,
    };
    var next = state;
    for (final rootId in rootIds) {
      if (rootId == ref.read(libraryUpdateRetainedRootProvider)) {
        continue;
      }
      final root = configuredRoots[rootId];
      if (root == null) {
        continue;
      }
      if (root.availability != LibraryRootAvailability.available ||
          next.tasksByRootId[root.id]?.isActive == true ||
          _pendingRoots.containsKey(root.id) ||
          _activeRuns.containsKey(root.id) ||
          !_tryAcquireRootUpdate(root.id)) {
        continue;
      }
      final scanId =
          "ame-update-${DateTime.now().microsecondsSinceEpoch}-${++_scanSequence}";
      next = next.withTask(LibraryRootUpdateTask(root: root, scanId: scanId));
      _pendingRoots[root.id] = root;
      _pendingRootIds.add(root.id);
    }
    state = next;
    _drainPendingUpdates();
  }

  void cancel(String rootId) {
    final task = state.tasksByRootId[rootId];
    if (task == null || !task.canCancel) {
      return;
    }
    if (_pendingRoots.remove(rootId) != null) {
      _pendingRootIds.remove(rootId);
      _releaseRootUpdate(rootId);
      state = state.withTask(
        task.copyWith(phase: LibraryRootUpdatePhase.cancelled),
      );
      return;
    }
    final run = _activeRuns[rootId];
    if (run != null) {
      run.cancelRequested = true;
      state = state.withTask(
        task.copyWith(phase: LibraryRootUpdatePhase.cancelling),
      );
      ref.read(libraryScannerProvider).cancel(run.scanId);
    }
  }

  void retry(String rootId) {
    final task = state.tasksByRootId[rootId];
    if (task == null || !task.canRetry) {
      return;
    }
    if (task.phase == LibraryRootUpdatePhase.refreshFailed) {
      final configuredRoot = _configuredRoot(rootId);
      if (configuredRoot == null ||
          configuredRoot.path != task.root.path ||
          configuredRoot.availability != LibraryRootAvailability.available ||
          !_tryAcquireRootUpdate(rootId)) {
        state = state.withTask(
          task.copyWith(
            errorMessage: "library_update_root_changed: 图库文件夹已移除、不可用或发生变化",
          ),
        );
        return;
      }
      state = state.withTask(
        task.copyWith(
          phase: LibraryRootUpdatePhase.refreshing,
          errorMessage: null,
        ),
      );
      _scheduleCatalogRefresh(task.root.id, task.scanId);
      return;
    }
    startUpdates([task.root.id]);
  }

  void dismiss(String rootId) {
    final task = state.tasksByRootId[rootId];
    if (task != null && !task.isActive) {
      state = state.withoutRoot(rootId);
    }
  }

  void dismissTerminalTasks() {
    state = state.withoutTerminalTasks();
  }

  void _drainPendingUpdates() {
    if (_isDisposed || _isClosing) {
      return;
    }
    while (_activeRuns.length < libraryUpdateMaxConcurrentRoots &&
        _pendingRootIds.isNotEmpty) {
      final rootId = _pendingRootIds.removeFirst();
      final root = _pendingRoots.remove(rootId);
      if (root != null) {
        _startUpdate(root);
      }
    }
  }

  void _startUpdate(LibraryRoot root) {
    final task = state.tasksByRootId[root.id];
    if (task == null || !task.isActive) {
      return;
    }
    final configuredRoot = _configuredRoot(root.id);
    if (configuredRoot == null ||
        configuredRoot.path != root.path ||
        configuredRoot.availability != LibraryRootAvailability.available) {
      state = state.withTask(
        task.copyWith(
          phase: LibraryRootUpdatePhase.failed,
          errorMessage: "library_update_root_changed: 图库文件夹已移除、不可用或发生变化",
        ),
      );
      _releaseRootUpdate(root.id);
      return;
    }
    final run = _ActiveRootUpdate(root: configuredRoot, scanId: task.scanId);
    _activeRuns[root.id] = run;
    state = state.withTask(
      task.copyWith(
        phase: LibraryRootUpdatePhase.discovering,
        errorMessage: null,
      ),
    );
    try {
      final stream = ref
          .read(libraryScannerProvider)
          .scan(
            scanId: run.scanId,
            rootPath: root.path,
            itemLimit: null,
            entryLimit: null,
            previewEdge: _previewEdge,
          );
      run.subscription = stream.listen(
        (update) => _handleUpdate(run, update),
        onError: (Object error, StackTrace _) {
          _handleStreamError(run, error);
        },
        onDone: () => _handleDone(run),
        cancelOnError: false,
      );
    } on Object catch (error) {
      _finishRun(
        run,
        task.copyWith(
          phase: LibraryRootUpdatePhase.failed,
          errorMessage: error.toString(),
        ),
      );
    }
  }

  void _handleUpdate(_ActiveRootUpdate run, LibraryScanUpdate update) {
    if (!_ownsRun(run)) {
      return;
    }
    if (run.terminalUpdate != null) {
      return;
    }
    final task = state.tasksByRootId[run.root.id];
    if (task == null) {
      return;
    }
    if (update case LibraryScanStarted(
      :final scanId,
    ) when scanId != run.scanId) {
      _handleStreamError(
        run,
        const LibraryScanFailure(
          code: "bridge_scan_id_mismatch",
          message: "Received a start event for a different scan",
        ),
      );
      return;
    }
    if (update is LibraryScanStarted && run.cancelRequested) {
      ref.read(libraryScannerProvider).cancel(run.scanId);
    }
    final next = switch (update) {
      LibraryScanStarted() => task.copyWith(
        phase: LibraryRootUpdatePhase.discovering,
      ),
      LibraryScanProgress(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.discovering,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        ),
      LibraryScanFinalizing(
        :final validatedItems,
        :final totalItems,
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.finalizing,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
          validatedItems: validatedItems,
          validationItemCount: totalItems,
        ),
      LibraryScanCompleted(:final assetCount, :final issueCount) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.refreshing,
          acceptedItems: assetCount,
          issueCount: issueCount,
        ),
      LibraryScanCancelled(:final acceptedItems, :final issueCount) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.cancelled,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        ),
      LibraryScanPaused(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.failed,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
          errorMessage: "图库更新已暂停，请重新开始更新。",
        ),
      LibraryScanStale(:final acceptedItems, :final issueCount) =>
        task.copyWith(
          phase: LibraryRootUpdatePhase.stale,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
          errorMessage: "更新期间源文件发生变化，请重试。",
        ),
      LibraryScanFailed(:final code, :final message) => task.copyWith(
        phase: LibraryRootUpdatePhase.failed,
        errorMessage: "$code: $message",
      ),
      LibraryAssetDiscovered() => task.copyWith(
        acceptedItems: task.acceptedItems + 1,
      ),
      LibraryIssueDiscovered() => task.copyWith(
        issueCount: task.issueCount + 1,
      ),
    };
    final publishedNext = run.cancelRequested && !_isTerminalUpdate(update)
        ? next.copyWith(phase: LibraryRootUpdatePhase.cancelling)
        : next;
    if (_isTerminalUpdate(update)) {
      run.terminalUpdate = update;
      run.terminalTask = publishedNext;
      return;
    }
    state = state.withTask(publishedNext);
  }

  void _handleStreamError(_ActiveRootUpdate run, Object error) {
    if (!_ownsRun(run)) {
      return;
    }
    run.streamError = error;
    run.cancelRequested = true;
    final task = state.tasksByRootId[run.root.id];
    if (task != null) {
      state = state.withTask(
        task.copyWith(phase: LibraryRootUpdatePhase.cancelling),
      );
    }
    ref.read(libraryScannerProvider).cancel(run.scanId);
  }

  void _handleDone(_ActiveRootUpdate run) {
    if (!_ownsRun(run)) {
      if (!run.streamDone.isCompleted) {
        run.streamDone.complete();
      }
      return;
    }
    final terminalTask = run.terminalTask;
    if (terminalTask != null) {
      final wasCompleted = run.terminalUpdate is LibraryScanCompleted;
      final releasedTask =
          run.streamError != null && run.terminalUpdate is LibraryScanCancelled
          ? terminalTask.copyWith(
              phase: LibraryRootUpdatePhase.failed,
              errorMessage: run.streamError.toString(),
            )
          : terminalTask;
      _finishRun(
        run,
        _isClosing
            ? releasedTask.copyWith(
                phase: LibraryRootUpdatePhase.cancelled,
                errorMessage: null,
              )
            : releasedTask,
      );
      if (wasCompleted && !_isClosing) {
        _scheduleCatalogRefresh(run.root.id, run.scanId);
      }
      return;
    }
    final task = state.tasksByRootId[run.root.id];
    if (task == null) {
      _releaseRun(run);
      return;
    }
    _finishRun(
      run,
      task.copyWith(
        phase: _isClosing
            ? LibraryRootUpdatePhase.cancelled
            : LibraryRootUpdatePhase.failed,
        errorMessage: _isClosing
            ? null
            : run.streamError?.toString() ?? "scan_stream_ended: 图库更新未返回完成结果",
      ),
    );
  }

  void _finishRun(_ActiveRootUpdate run, LibraryRootUpdateTask task) {
    if (!identical(_activeRuns[run.root.id], run)) {
      return;
    }
    _activeRuns.remove(run.root.id);
    run.cancelSubscription();
    if (!task.isActive) {
      _releaseRootUpdate(run.root.id);
    }
    state = state.withTask(task);
    if (!run.streamDone.isCompleted) {
      run.streamDone.complete();
    }
    _drainPendingUpdates();
  }

  void _releaseRun(_ActiveRootUpdate run) {
    if (identical(_activeRuns[run.root.id], run)) {
      _activeRuns.remove(run.root.id);
      run.cancelSubscription();
      _releaseRootUpdate(run.root.id);
    }
    if (!run.streamDone.isCompleted) {
      run.streamDone.complete();
    }
    _drainPendingUpdates();
  }

  bool _ownsRun(_ActiveRootUpdate run) {
    return !_isDisposed && identical(_activeRuns[run.root.id], run);
  }

  LibraryRoot? _configuredRoot(String rootId) {
    for (final root in ref.read(libraryUpdateConfiguredRootsProvider)) {
      if (root.id == rootId) {
        return root;
      }
    }
    return null;
  }

  bool _isTerminalUpdate(LibraryScanUpdate update) {
    return update is LibraryScanCompleted ||
        update is LibraryScanCancelled ||
        update is LibraryScanPaused ||
        update is LibraryScanStale ||
        update is LibraryScanFailed;
  }

  void _scheduleCatalogRefresh(String rootId, String scanId) {
    final refresh = ref.read(libraryUpdateCatalogRefreshProvider);
    _catalogRefreshQueue = _catalogRefreshQueue.then((_) async {
      if (_isDisposed || _isClosing) {
        return;
      }
      try {
        await refresh();
        if (_isDisposed || _isClosing) {
          return;
        }
        ref
            .read(libraryControllerProvider.notifier)
            .restorePreviewAuthority(rootId);
        final task = state.tasksByRootId[rootId];
        if (task != null &&
            task.scanId == scanId &&
            task.phase == LibraryRootUpdatePhase.refreshing) {
          state = state.withTask(
            task.copyWith(phase: LibraryRootUpdatePhase.completed),
          );
        }
      } on Object catch (error) {
        if (_isDisposed || _isClosing) {
          return;
        }
        final task = state.tasksByRootId[rootId];
        if (task != null &&
            task.scanId == scanId &&
            task.phase == LibraryRootUpdatePhase.refreshing) {
          state = state.withTask(
            task.copyWith(
              phase: LibraryRootUpdatePhase.refreshFailed,
              errorMessage: "图库已更新，但刷新显示失败：$error",
            ),
          );
        }
      } finally {
        _releaseRootUpdate(rootId);
      }
    });
  }

  Future<void> _cancelActiveUpdatesForShutdown() async {
    if (_isDisposed || _isClosing) {
      return;
    }
    final scanExecutionCoordinator = _scanExecutionCoordinator;
    if (scanExecutionCoordinator == null) {
      return;
    }
    _isClosing = true;
    var next = state;
    for (final rootId in _pendingRootIds) {
      final task = next.tasksByRootId[rootId];
      if (task != null) {
        next = next.withTask(
          task.copyWith(phase: LibraryRootUpdatePhase.cancelled),
        );
      }
      _releaseRootUpdate(rootId);
    }
    for (final task in next.tasks) {
      if (task.phase == LibraryRootUpdatePhase.refreshing) {
        next = next.withTask(
          task.copyWith(phase: LibraryRootUpdatePhase.cancelled),
        );
        _releaseRootUpdate(task.root.id);
      }
    }
    _pendingRootIds.clear();
    _pendingRoots.clear();
    final scanner = ref.read(libraryScannerProvider);
    final runs = _activeRuns.values.toList(growable: false);
    for (final run in runs) {
      run.cancelRequested = true;
      final task = next.tasksByRootId[run.root.id];
      if (task != null && task.canCancel) {
        next = next.withTask(
          task.copyWith(phase: LibraryRootUpdatePhase.cancelling),
        );
      }
      scanner.cancel(run.scanId);
    }
    state = next;
    await Future.wait(runs.map((run) => run.streamDone.future));
  }

  LibraryScanExecutionCoordinator _readScanExecutionCoordinator() {
    final scanExecutionCoordinator = _scanExecutionCoordinator;
    if (scanExecutionCoordinator != null) {
      return scanExecutionCoordinator;
    }
    final initialized = ref.read(libraryScanExecutionCoordinatorProvider);
    _scanExecutionCoordinator = initialized;
    return initialized;
  }

  bool _tryAcquireRootUpdate(String rootId) {
    final scanExecutionCoordinator = _readScanExecutionCoordinator();
    if (!scanExecutionCoordinator.tryAcquireUpdate(rootId)) {
      return false;
    }
    _ownedRootIds.add(rootId);
    return true;
  }

  void _releaseRootUpdate(String rootId) {
    if (!_ownedRootIds.remove(rootId)) {
      return;
    }
    _scanExecutionCoordinator?.releaseUpdate(rootId);
  }
}

final libraryUpdateControllerProvider =
    NotifierProvider<LibraryUpdateController, LibraryUpdateState>(
      LibraryUpdateController.new,
    );
