import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../application/library_controller.dart";
import "../../application/library_browse_admission.dart";
import "../../application/library_classification_lifecycle.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_publication.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../application/library_similar_analysis_scheduler.dart";
import "../../application/library_synchronization.dart";
import "../../application/library_update_controller.dart";
import "../../domain/library_synchronization_models.dart";
import "../../domain/library_models.dart";
import "../library_classification_notifications.dart";

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
  LibrarySimilarAnalysisScheduler? _similarity;
  LibraryClassificationLifecycle? _classification;
  late final LibraryExactAnalysisPublication _publication;
  late final LibraryExactAnalysisController _analysis;
  StreamSubscription<LibrarySynchronizationSnapshot>? _synchronization;
  late LibrarySynchronizationSnapshot _snapshot;

  @override
  void initState() {
    super.initState();
    _analysis = ref.read(libraryExactAnalysisControllerProvider);
    _publication = LibraryExactAnalysisPublication(
      _analysis,
      canRefresh: () => LibraryBrowseAdmission(
        ref.read(libraryControllerProvider),
      ).canReadPage,
      refresh: () =>
          ref.read(libraryControllerProvider.notifier).refreshCurrentQuery(),
    );
    _scheduler = ref.read(libraryExactAnalysisSchedulerProvider);
    _similarity = ref.read(librarySimilarAnalysisSchedulerProvider);
    _classification = ref.read(libraryClassificationLifecycleProvider);
    final notifications = LibraryClassificationNotifications(
      ref.read(ameNotificationControllerProvider.notifier),
    );
    ref.listenManual(
      libraryClassificationLifecycleProvider.select((owner) => owner?.error),
      (_, _) => Future<void>.microtask(() {
        if (mounted) notifications.accept(_classification?.error);
      }),
      fireImmediately: true,
    );
    final synchronization = ref.read(librarySynchronizationProvider);
    _snapshot = synchronization.current;
    _updateCatalog();
    if (_scheduler == null && _classification == null && _similarity == null) {
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
    HardwareKeyboard.instance.addHandler(_onKey);
  }

  void _updateCatalog() {
    final state = ref.read(libraryControllerProvider);
    final updates = ref.read(libraryUpdateControllerProvider);
    final revision = state.catalogRevision;
    final currentRevision =
        revision == null || revision < _snapshot.catalogRevision
        ? _snapshot.catalogRevision
        : revision;
    final roots = {
      for (final root in state.roots)
        root.id: _snapshot.roots[root.id]?.availability ?? root.availability,
    };
    final isBusy =
        state.isTaskProcessing ||
        state.isCommittedRemovalReloadPending ||
        updates.hasActive;
    _similarity?.updateCatalog(
      revision: currentRevision,
      roots: roots,
      isBusy: isBusy,
    );
    _scheduler?.updateCatalog(
      revision: currentRevision,
      roots: roots,
      isBusy: isBusy,
    );
    _classification?.updateCatalog(
      revision: currentRevision,
      availableRoots: {
        for (final root in _snapshot.roots.values)
          if (root.availability == LibraryRootAvailability.available)
            root.rootId: root.rootGeneration,
      },
      isBusy: isBusy,
    );
    if (!state.isBusy && state.errorMessage == null) {
      _publication.retry();
    }
  }

  bool _onKey(KeyEvent event) {
    if (event is KeyDownEvent || event is KeyRepeatEvent) {
      _browsingActivity();
    }
    return false;
  }

  @override
  void dispose() {
    _publication.dispose();
    if (_scheduler != null || _classification != null || _similarity != null) {
      HardwareKeyboard.instance.removeHandler(_onKey);
    }
    unawaited(_synchronization?.cancel());
    super.dispose();
  }

  void _browsingActivity() {
    _scheduler?.browsingActivity();
    _similarity?.browsingActivity();
    _classification?.browsingActivity();
  }

  @override
  Widget build(BuildContext context) => Listener(
    onPointerDown: (_) => _browsingActivity(),
    onPointerMove: (_) => _browsingActivity(),
    onPointerSignal: (_) => _browsingActivity(),
    child: widget.child,
  );
}
