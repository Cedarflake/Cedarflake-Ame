import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization_archive.dart";
import "library_album_pages.dart";
import "library_archive_requests.dart";
import "library_archive_state.dart";
import "library_organization_archive.dart";
import "library_organization_publication.dart";

final libraryArchiveControllerProvider =
    NotifierProvider<LibraryArchiveController, LibraryArchiveState>(
      LibraryArchiveController.new,
    );

class LibraryArchiveController extends Notifier<LibraryArchiveState> {
  LibraryArchiveTask? _active;
  bool _disposed = false;

  @override
  LibraryArchiveState build() {
    ref.onDispose(() {
      _disposed = true;
      _active?.cancel();
    });
    return const LibraryArchiveIdle();
  }

  Future<void> export(String path) => _run(LibraryArchiveExport(path));
  Future<void> inspect(String path) => _run(LibraryArchiveInspect(path));

  Future<void> restore(LibraryArchiveReady observed) async {
    if (!identical(state, observed)) return;
    await _run(LibraryArchiveRestore(observed.path, observed.inspection));
  }

  Future<void> retry() async {
    final failed = state;
    if (failed is! LibraryArchiveFailed) return;
    if (failed.failure.requiresInspection) {
      await inspect(failed.request.path);
    } else {
      await _run(failed.request);
    }
  }

  void dismiss() {
    if (!_disposed && _active == null) state = const LibraryArchiveIdle();
  }

  void cancel() {
    final current = state;
    final task = _active;
    if (current is! LibraryArchiveWorking || current.stopping || task == null)
      return;
    try {
      task.cancel();
      state = LibraryArchiveWorking(
        current.request,
        phase: current.phase,
        stopping: true,
      );
    } on Object {
      state = LibraryArchiveWorking(
        current.request,
        phase: current.phase,
        cancelFailed: true,
      );
    }
  }

  Future<void> _run(LibraryArchiveRequest request) async {
    if (_disposed || _active != null) return;
    state = LibraryArchiveWorking(request);
    LibraryArchiveTask? task;
    LibraryArchiveEvent? result;
    LibraryArchiveFailure? failure;
    try {
      task = ref.read(libraryOrganizationArchivesProvider).createTask();
      _active = task;
      await for (final event in request.start(task)) {
        if (_disposed) break;
        if (event case LibraryArchiveProgress(:final phase)) {
          final current = state;
          if (current is LibraryArchiveWorking && current.phase != phase) {
            state = LibraryArchiveWorking(
              request,
              phase: phase,
              stopping: current.stopping,
              cancelFailed: current.cancelFailed,
            );
          }
          continue;
        }
        if (event is! LibraryArchiveCancelled && !request.accepts(event)) {
          throw const LibraryArchiveFailure(
            "archive_result_unexpected",
            "The backup task returned a different operation result",
          );
        }
        result = event;
        break;
      }
      if (result == null && !_disposed) {
        throw const LibraryArchiveFailure(
          "archive_result_missing",
          "The backup task ended without a confirmed result",
        );
      }
    } on Object catch (error) {
      failure = error is LibraryArchiveFailure
          ? error
          : LibraryArchiveFailure("archive_failed", error.toString());
      debugPrint(
        "[Ame organization archive] outcome=failed code=${failure.code}",
      );
    } finally {
      if (result == null && !_disposed && task != null) {
        try {
          task.cancel();
        } on Object catch (error) {
          debugPrint("[Ame organization archive] cancellation failed: $error");
        }
      }
      task?.dispose();
      if (identical(_active, task)) _active = null;
    }
    if (_disposed) return;
    if (failure != null) {
      state = LibraryArchiveFailed(request, failure);
      return;
    }
    switch (result) {
      case LibraryArchiveInspected(:final inspection):
        state = LibraryArchiveReady(request.path, inspection);
      case LibraryArchiveCancelled():
        state = LibraryArchiveFinished(request, cancelled: true);
      case LibraryArchiveExported():
        state = LibraryArchiveFinished(request);
        _notify("整理记录已导出", "备份已保存到所选位置。");
      case LibraryArchiveRestored():
        state = LibraryArchiveFinished(request);
        ref.invalidate(libraryAlbumPageProvider);
        ref.invalidate(libraryAlbumProvider);
        _notify("整理记录已恢复", "暂未关联的记录已保留。");
        await ref.read(libraryOrganizationPublicationProvider).refresh();
      case LibraryArchiveProgress() || null:
        throw StateError("Archive completion requires a terminal result");
    }
  }

  void _notify(String title, String message) {
    ref
        .read(ameNotificationControllerProvider.notifier)
        .publish(
          AmeNotificationDraft(
            title: title,
            message: message,
            severity: AmeNotificationSeverity.success,
            dedupeKey: "organization-archive-result",
          ),
        );
  }
}
