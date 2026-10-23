import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../application/library_controller.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_publication.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../application/library_synchronization.dart";
import "../../application/library_update_controller.dart";
import "../../domain/library_synchronization_models.dart";
import "../library_results_source_activity.dart";
import "library_results_progress.dart";

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
  late final LibraryExactAnalysisPublication _publication;
  late final LibraryExactAnalysisController _analysis;
  late final Listenable _progress;
  final _sources = LibraryResultsSourceActivity();
  StreamSubscription<LibrarySynchronizationSnapshot>? _synchronization;
  late LibrarySynchronizationSnapshot _snapshot;

  @override
  void initState() {
    super.initState();
    _analysis = ref.read(libraryExactAnalysisControllerProvider);
    _publication = LibraryExactAnalysisPublication(
      _analysis,
      refresh: () =>
          ref.read(libraryControllerProvider.notifier).refreshCurrentQuery(),
    );
    _scheduler = ref.read(libraryExactAnalysisSchedulerProvider);
    _progress = Listenable.merge([_analysis, _publication, _scheduler]);
    final synchronization = ref.read(librarySynchronizationProvider);
    _snapshot = synchronization.current;
    _progress.addListener(_updateProgress);
    _updateCatalog();
    if (_scheduler == null) {
      return;
    }
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
          state.isTaskProcessing ||
          state.isCommittedRemovalReloadPending ||
          updates.hasActive,
    );
    if (!state.isBusy && state.errorMessage == null) {
      _publication.retry();
    }
    _updateProgress();
  }

  void _updateProgress() {
    final state = ref.read(libraryControllerProvider);
    _sources.update(
      roots: state.roots,
      synchronization: _snapshot,
      updatingRootIds: {
        ...ref.read(libraryUpdateControllerProvider).activeRootIds,
        if (state.isTaskProcessing && state.retainedScanRootId != null)
          state.retainedScanRootId!,
      },
      progress: LibraryResultsProgress.resolve(
        analysis: _analysis,
        publication: _publication,
        scheduler: _scheduler,
        displayedRevision: state.catalogRevision,
        expectedRevision: _snapshot.catalogRevision,
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
    _progress.removeListener(_updateProgress);
    _sources.dispose();
    _publication.dispose();
    if (_scheduler != null) {
      HardwareKeyboard.instance.removeHandler(_onKey);
    }
    unawaited(_synchronization?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _sources,
    builder: (context, _) => LibraryResultsProgressScope(
      progress: _sources.progress,
      rootIds: _sources.rootIds,
      child: Listener(
        onPointerDown: (_) => _scheduler?.browsingActivity(),
        onPointerMove: (_) => _scheduler?.browsingActivity(),
        onPointerSignal: (_) => _scheduler?.browsingActivity(),
        child: widget.child,
      ),
    ),
  );
}
