import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../application/library_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../application/library_synchronization.dart";
import "../../application/library_update_controller.dart";
import "../../domain/library_synchronization_models.dart";

/// Bridges catalog publication and foreground input to the analysis scheduler.
class LibraryAnalysisActivity extends ConsumerStatefulWidget {
  const LibraryAnalysisActivity({required this.child, super.key});
  final Widget child;

  @override
  ConsumerState<LibraryAnalysisActivity> createState() =>
      _LibraryAnalysisActivityState();
}

class _LibraryAnalysisActivityState
    extends ConsumerState<LibraryAnalysisActivity> {
  LibraryExactAnalysisScheduler? _scheduler;
  StreamSubscription<LibrarySynchronizationSnapshot>? _synchronization;
  late LibrarySynchronizationSnapshot _snapshot;

  @override
  void initState() {
    super.initState();
    _scheduler = ref.read(libraryExactAnalysisSchedulerProvider);
    if (_scheduler == null) {
      return;
    }
    final synchronization = ref.read(librarySynchronizationProvider);
    _snapshot = synchronization.current;
    _synchronization = synchronization.watch().listen((snapshot) {
      _snapshot = snapshot;
      _updateCatalog();
    });
    ref.listenManual(libraryControllerProvider, (_, _) => _updateCatalog());
    ref.listenManual(
      libraryUpdateControllerProvider,
      (_, _) => _updateCatalog(),
    );
    ref.listenManual(
      amePreferencesControllerProvider.select(
        (value) => value.exactAnalysisPaused,
      ),
      (_, paused) => _scheduler?.setPaused(paused),
      fireImmediately: true,
    );
    HardwareKeyboard.instance.addHandler(_onKey);
    _updateCatalog();
  }

  void _updateCatalog() {
    final state = ref.read(libraryControllerProvider);
    final updates = ref.read(libraryUpdateControllerProvider);
    final revision = state.catalogRevision;
    _scheduler?.updateCatalog(
      revision: revision == null || revision < _snapshot.catalogRevision
          ? _snapshot.catalogRevision
          : revision,
      roots: {
        for (final root in state.roots)
          root.id: _snapshot.roots[root.id]?.availability ?? root.availability,
      },
      isBusy:
          state.isBusy ||
          updates.hasActive ||
          state.isLoadingTimeAnchor ||
          state.isLoadingVisibleRange ||
          _snapshot.roots.values.any(
            (root) => root.freshness == LibraryCatalogFreshness.updating,
          ),
    );
  }

  bool _onKey(KeyEvent event) {
    if (event is KeyDownEvent || event is KeyRepeatEvent) {
      _scheduler?.browsingActivity();
    }
    return false;
  }

  @override
  void dispose() {
    if (_scheduler != null) {
      HardwareKeyboard.instance.removeHandler(_onKey);
    }
    unawaited(_synchronization?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Listener(
    onPointerDown: (_) => _scheduler?.browsingActivity(),
    onPointerMove: (_) => _scheduler?.browsingActivity(),
    onPointerSignal: (_) => _scheduler?.browsingActivity(),
    child: widget.child,
  );
}
