import "../domain/library_organization_cleanup.dart";
import "library_organization_cleanup.dart";

sealed class LibraryCleanupRequest {
  LibraryCleanupRequest(this.minimumAgeDays);
  final int? minimumAgeDays;
  Stream<LibraryCleanupEvent> start(LibraryCleanupTask task);
  bool accepts(LibraryCleanupEvent event);
}

final class LibraryCleanupInspect extends LibraryCleanupRequest {
  LibraryCleanupInspect(super.minimumAgeDays)
    : cutoffUnixMs = minimumAgeDays == null
          ? null
          : DateTime.now()
                .subtract(Duration(days: minimumAgeDays))
                .millisecondsSinceEpoch;
  final int? cutoffUnixMs;
  @override
  Stream<LibraryCleanupEvent> start(LibraryCleanupTask task) =>
      task.inspect(cutoffUnixMs);
  @override
  bool accepts(LibraryCleanupEvent event) => event is LibraryCleanupInspected;
}

final class LibraryCleanupApply extends LibraryCleanupRequest {
  LibraryCleanupApply(super.minimumAgeDays, this.inspection)
    : operationId =
          "cleanup-${DateTime.now().microsecondsSinceEpoch}-${++_sequence}";
  static int _sequence = 0;
  final String operationId;
  final LibraryCleanupInspection inspection;
  @override
  Stream<LibraryCleanupEvent> start(LibraryCleanupTask task) =>
      task.apply(operationId, inspection);
  @override
  bool accepts(LibraryCleanupEvent event) =>
      event is LibraryCleanupCompleted && event.operationId == operationId;
}

sealed class LibraryCleanupState {
  const LibraryCleanupState();
}

final class LibraryCleanupIdle extends LibraryCleanupState {
  const LibraryCleanupIdle();
}

final class LibraryCleanupWorking extends LibraryCleanupState {
  const LibraryCleanupWorking(
    this.request, {
    this.phase = LibraryCleanupPhase.inspecting,
    this.stopping = false,
    this.cancelFailed = false,
  });
  final LibraryCleanupRequest request;
  final LibraryCleanupPhase phase;
  final bool stopping;
  final bool cancelFailed;
}

final class LibraryCleanupReady extends LibraryCleanupState {
  const LibraryCleanupReady(this.minimumAgeDays, this.inspection);
  final int? minimumAgeDays;
  final LibraryCleanupInspection inspection;
}

final class LibraryCleanupFailed extends LibraryCleanupState {
  const LibraryCleanupFailed(this.request, this.failure);
  final LibraryCleanupRequest request;
  final LibraryCleanupFailure failure;
}

final class LibraryCleanupFinished extends LibraryCleanupState {
  const LibraryCleanupFinished({
    this.removedRecords = 0,
    this.cancelled = false,
  });
  final int removedRecords;
  final bool cancelled;
}
