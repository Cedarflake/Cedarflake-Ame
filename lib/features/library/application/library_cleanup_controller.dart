import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization_cleanup.dart";
import "library_cleanup_state.dart";
import "library_organization_cleanup.dart";
import "library_organization_publication.dart";

final libraryCleanupControllerProvider =
    NotifierProvider<LibraryCleanupController, LibraryCleanupState>(
      LibraryCleanupController.new,
    );

class LibraryCleanupController extends Notifier<LibraryCleanupState> {
  LibraryCleanupTask? _active;
  bool _disposed = false;

  @override
  LibraryCleanupState build() {
    ref.onDispose(() {
      _disposed = true;
      _requestCancellation(_active);
    });
    return const LibraryCleanupIdle();
  }

  Future<void> inspect(int? minimumAgeDays) =>
      _run(LibraryCleanupInspect(minimumAgeDays));

  Future<void> apply(LibraryCleanupReady observed) async {
    if (!identical(state, observed) || observed.inspection.total == 0) return;
    await _run(
      LibraryCleanupApply(observed.minimumAgeDays, observed.inspection),
    );
  }

  Future<void> retry() async {
    final failed = state;
    if (failed is! LibraryCleanupFailed) return;
    if (failed.failure.requiresInspection) {
      await inspect(failed.request.minimumAgeDays);
    } else {
      await _run(failed.request);
    }
  }

  void dismiss() {
    if (!_disposed && _active == null) state = const LibraryCleanupIdle();
  }

  void cancel() {
    final current = state;
    if (current is! LibraryCleanupWorking ||
        current.stopping ||
        _active == null) {
      return;
    }
    final requested = _requestCancellation(_active);
    state = LibraryCleanupWorking(
      current.request,
      phase: current.phase,
      stopping: requested,
      cancelFailed: !requested,
    );
  }

  bool _requestCancellation(LibraryCleanupTask? task) {
    try {
      task?.cancel();
      return true;
    } on Object catch (error) {
      debugPrint("[Ame organization cleanup] cancellation failed: $error");
      return false;
    }
  }

  Future<void> _run(LibraryCleanupRequest request) async {
    if (_disposed || _active != null) return;
    state = LibraryCleanupWorking(request);
    LibraryCleanupTask? task;
    LibraryCleanupEvent? result;
    LibraryCleanupFailure? failure;
    try {
      task = ref.read(libraryOrganizationCleanupProvider).createTask();
      _active = task;
      await for (final event in request.start(task)) {
        if (_disposed) break;
        if (event case LibraryCleanupProgress(:final phase)) {
          final current = state;
          if (current is LibraryCleanupWorking && current.phase != phase) {
            state = LibraryCleanupWorking(
              request,
              phase: phase,
              stopping: current.stopping,
              cancelFailed: current.cancelFailed,
            );
          }
          continue;
        }
        if (event is! LibraryCleanupCancelled && !request.accepts(event)) {
          throw const LibraryCleanupFailure(
            "cleanup_result_unexpected",
            "The cleanup task returned a different operation result",
          );
        }
        result = event;
        break;
      }
      if (result == null && !_disposed) {
        throw const LibraryCleanupFailure(
          "cleanup_result_missing",
          "The cleanup task ended without a confirmed result",
        );
      }
    } on Object catch (error) {
      failure = error is LibraryCleanupFailure
          ? error
          : LibraryCleanupFailure("cleanup_failed", error.toString());
      debugPrint(
        "[Ame organization cleanup] outcome=failed code=${failure.code}",
      );
    } finally {
      if (result == null) _requestCancellation(task);
      task?.dispose();
      if (identical(_active, task)) _active = null;
    }
    if (_disposed) return;
    if (failure != null) {
      state = LibraryCleanupFailed(request, failure);
      return;
    }
    switch (result) {
      case LibraryCleanupInspected(:final inspection):
        state = LibraryCleanupReady(request.minimumAgeDays, inspection);
      case LibraryCleanupCancelled():
        state = const LibraryCleanupFinished(cancelled: true);
      case LibraryCleanupCompleted(:final removedRecords):
        state = LibraryCleanupFinished(removedRecords: removedRecords);
        ref
            .read(ameNotificationControllerProvider.notifier)
            .publish(
              AmeNotificationDraft(
                title: "已清理未关联记录",
                message: "已清理 $removedRecords 项记录。原图片和相册均已保留。",
                severity: AmeNotificationSeverity.success,
                dedupeKey: "organization-cleanup-result",
              ),
            );
        await ref.read(libraryOrganizationPublicationProvider).refresh();
      case LibraryCleanupProgress() || null:
        throw StateError("Cleanup completion requires a terminal result");
    }
  }
}
