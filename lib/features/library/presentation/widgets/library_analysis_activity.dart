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
    _updateCatalog();
  }

  void _updateCatalog() {
    final state = ref.read(libraryControllerProvider);
    final updates = ref.read(libraryUpdateControllerProvider);
    if (!state.isBusy && state.errorMessage == null) {
      _publication.retry();
    }
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
  }

  bool _onKey(KeyEvent event) {
    if (event is KeyDownEvent || event is KeyRepeatEvent) {
      _scheduler?.browsingActivity();
    }
    return false;
  }

  @override
  void dispose() {
    _publication.dispose();
    if (_scheduler != null) {
      HardwareKeyboard.instance.removeHandler(_onKey);
    }
    unawaited(_synchronization?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _progress,
    builder: (context, _) => LibraryResultsProgressScope(
      progress: LibraryResultsProgress.resolve(
        analysis: _analysis,
        publication: _publication,
        scheduler: _scheduler,
        displayedRevision: ref.read(libraryControllerProvider).catalogRevision,
        expectedRevision: _snapshot.catalogRevision,
      ),
      rootIds: {
        for (final root in ref.read(libraryControllerProvider).roots) root.id,
      },
      child: Listener(
        onPointerDown: (_) => _scheduler?.browsingActivity(),
        onPointerMove: (_) => _scheduler?.browsingActivity(),
        onPointerSignal: (_) => _scheduler?.browsingActivity(),
        child: widget.child,
      ),
    ),
  );
}
