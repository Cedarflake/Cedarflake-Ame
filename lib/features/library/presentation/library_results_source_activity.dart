import "package:flutter/foundation.dart";

import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";
import "widgets/library_results_progress.dart";

typedef _SourceVersion = ({
  String path,
  String? scanId,
  int assetCount,
  int issueCount,
  LibraryRootAvailability availability,
});

/// Retains affected sources until analysis and visible publication both settle.
class LibraryResultsSourceActivity extends ChangeNotifier {
  Map<String, _SourceVersion> _versions = const {};
  Map<String, BigInt> _generations = const {};
  final Set<String> _pending = {};
  Set<String> _rootIds = const {};
  LibraryResultsProgress _progress = LibraryResultsProgress.ready;

  Set<String> get rootIds => _rootIds;
  LibraryResultsProgress get progress => _progress;

  void update({
    required List<LibraryRoot> roots,
    required LibrarySynchronizationSnapshot synchronization,
    required Set<String> updatingRootIds,
    required LibraryResultsProgress progress,
  }) {
    final versions = <String, _SourceVersion>{};
    final generations = <String, BigInt>{};
    final active = <String>{...updatingRootIds};
    for (final root in roots) {
      final status = synchronization.roots[root.id];
      final version = (
        path: root.path,
        scanId: root.activeScanId,
        assetCount: root.assetCount,
        issueCount: root.issueCount,
        availability: status?.availability ?? root.availability,
      );
      versions[root.id] = version;
      final generation = status?.rootGeneration;
      final previousGeneration = _generations[root.id];
      if (_versions[root.id] != version ||
          (previousGeneration != null &&
              generation != null &&
              previousGeneration != generation)) {
        _pending.add(root.id);
      }
      if (generation != null) {
        generations[root.id] = generation;
      }
      if (status != null &&
          (status.freshness == LibraryCatalogFreshness.updating ||
              status.pendingChangeCount > BigInt.zero ||
              status.retryWaitCount > BigInt.zero ||
              status.freshnessUnknownCount > BigInt.zero)) {
        active.add(root.id);
      }
    }
    _versions = versions;
    _generations = generations;
    _pending.addAll(active);
    _pending.retainAll(versions.keys);
    if (progress == LibraryResultsProgress.ready) {
      _pending.retainAll(active);
    }
    if (_progress == progress && setEquals(_rootIds, _pending)) {
      return;
    }
    _progress = progress;
    _rootIds = Set.unmodifiable(_pending);
    notifyListeners();
  }
}
