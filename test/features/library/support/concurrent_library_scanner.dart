import "dart:async";

import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";

class ConcurrentLibraryScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  ConcurrentLibraryScanner({
    this.completeCancellation = false,
    this.delayedRegistration = false,
  });

  final bool completeCancellation;
  final bool delayedRegistration;
  final Map<String, StreamController<LibraryScanUpdate>> _controllers = {};
  final Map<String, String> _rootPathsByScanId = {};
  final Set<String> _registeredScanIds = {};
  final List<String> startedRootPaths = [];
  final List<String> cancelledScanIds = [];

  Iterable<String> get activeScanIds => _controllers.keys;

  String scanIdForRootPath(String rootPath) {
    return _rootPathsByScanId.entries
        .singleWhere((entry) => entry.value == rootPath)
        .key;
  }

  void add(String scanId, LibraryScanUpdate update) {
    if (update is LibraryScanStarted) {
      _registeredScanIds.add(scanId);
    }
    _controllers[scanId]?.add(update);
  }

  void addError(String scanId, Object error) {
    _controllers[scanId]?.addError(error);
  }

  Future<void> close(String scanId) async {
    _registeredScanIds.remove(scanId);
    await _controllers.remove(scanId)?.close();
  }

  void dispose() {
    for (final controller in _controllers.values) {
      unawaited(controller.close());
    }
    _controllers.clear();
    _registeredScanIds.clear();
  }

  @override
  bool cancel(String scanId) {
    if (!_registeredScanIds.contains(scanId)) {
      return false;
    }
    cancelledScanIds.add(scanId);
    if (completeCancellation) {
      scheduleMicrotask(() {
        add(
          scanId,
          const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
        );
        unawaited(close(scanId));
      });
    }
    return true;
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    throw UnimplementedError();
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    // Every controller is closed by the registered scanner teardown.
    // ignore: close_sinks
    final controller = StreamController<LibraryScanUpdate>();
    _controllers[scanId] = controller;
    _rootPathsByScanId[scanId] = rootPath;
    startedRootPaths.add(rootPath);
    if (!delayedRegistration) {
      _registeredScanIds.add(scanId);
    }
    return controller.stream;
  }

  @override
  bool suspend(String scanId) => _controllers.containsKey(scanId);
}
