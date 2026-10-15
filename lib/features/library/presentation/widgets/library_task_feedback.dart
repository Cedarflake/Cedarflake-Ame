import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_controller.dart";
import "../../application/library_update_controller.dart";
import "../../domain/library_state.dart";
import "../library_task_surface_selection.dart";
import "library_task_surface.dart";
import "library_update_task_surface.dart";

typedef _Completion = ({String? rootId, String scanId});

class LibraryTaskFeedback extends ConsumerStatefulWidget {
  const LibraryTaskFeedback({required this.onRetryPrimary, super.key});

  final Future<void> Function() onRetryPrimary;

  @override
  ConsumerState<LibraryTaskFeedback> createState() =>
      _LibraryTaskFeedbackState();
}

class _LibraryTaskFeedbackState extends ConsumerState<LibraryTaskFeedback> {
  final Map<_Completion, Timer> _dismissTimers = {};
  bool _hasFocus = false;
  bool _isHovered = false;
  bool _accessibleNavigation = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _accessibleNavigation = MediaQuery.accessibleNavigationOf(context);
    _synchronizeCompletions();
  }

  Set<_Completion> _eligibleCompletions() {
    final state = ref.read(libraryControllerProvider);
    final updates = ref.read(libraryUpdateControllerProvider);
    final selection = LibraryTaskSurfaceSelection(state, updates);
    final completions = <_Completion>{};
    final primary = state.primaryScanSnapshot;
    if (selection.hasPrimary &&
        state.taskKind != LibraryTaskKind.remove &&
        primary.status == LibraryStatus.completed &&
        primary.scanId != null &&
        !primary.hasUnresolvedPublication &&
        !primary.isScanLimited) {
      completions.add((rootId: null, scanId: primary.scanId!));
    }
    for (final task in updates.tasks) {
      if (task.phase == LibraryRootUpdatePhase.completed) {
        completions.add((rootId: task.root.id, scanId: task.scanId));
      }
    }
    return completions;
  }

  void _synchronizeCompletions() {
    final visible = _accessibleNavigation || _hasFocus || _isHovered
        ? <_Completion>{}
        : _eligibleCompletions();
    for (final completion in _dismissTimers.keys.toList()) {
      if (!visible.contains(completion)) {
        _dismissTimers.remove(completion)?.cancel();
      }
    }
    for (final completion in visible) {
      _dismissTimers.putIfAbsent(
        completion,
        () => Timer(const Duration(seconds: 5), () {
          _dismissTimers.remove(completion);
          if (!mounted || !_eligibleCompletions().contains(completion)) return;
          final rootId = completion.rootId;
          if (rootId == null) {
            ref.read(libraryControllerProvider.notifier).dismissTaskFeedback();
          } else {
            ref.read(libraryUpdateControllerProvider.notifier).dismiss(rootId);
          }
        }),
      );
    }
  }

  @override
  void dispose() {
    for (final timer in _dismissTimers.values) {
      timer.cancel();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(libraryControllerProvider, (_, _) => _synchronizeCompletions());
    ref.listen(
      libraryUpdateControllerProvider,
      (_, _) => _synchronizeCompletions(),
    );
    final state = ref.watch(libraryControllerProvider);
    final updates = ref.watch(libraryUpdateControllerProvider);
    final controller = ref.read(libraryControllerProvider.notifier);
    final updateController = ref.read(libraryUpdateControllerProvider.notifier);
    final selection = LibraryTaskSurfaceSelection(state, updates);
    return MouseRegion(
      onEnter: (_) {
        _isHovered = true;
        _synchronizeCompletions();
      },
      onExit: (_) {
        _isHovered = false;
        _synchronizeCompletions();
      },
      child: Focus(
        canRequestFocus: false,
        onFocusChange: (hasFocus) {
          _hasFocus = hasFocus;
          _synchronizeCompletions();
        },
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (selection.showUpdates)
              Flexible(
                child: LibraryUpdateTaskSurface(
                  state: updates,
                  onCancel: updateController.cancel,
                  onRetry: updateController.retry,
                  onDismiss: updateController.dismiss,
                  onDismissTerminalTasks: updateController.dismissTerminalTasks,
                ),
              ),
            if (selection.showUpdates && selection.showPrimary)
              const SizedBox(height: 12),
            if (selection.showPrimary)
              LibraryTaskSurface(
                state: state,
                onPause: controller.pauseScan,
                onCancel: controller.cancelScan,
                onResume: controller.resumePausedScan,
                onRetry: widget.onRetryPrimary,
                onDismiss: controller.dismissTaskFeedback,
              ),
          ],
        ),
      ),
    );
  }
}
