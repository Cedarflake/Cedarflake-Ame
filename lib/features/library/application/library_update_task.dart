import "../domain/library_models.dart";

enum LibraryRootUpdatePhase {
  queued,
  discovering,
  finalizing,
  refreshing,
  cancelling,
  completed,
  cancelled,
  stale,
  refreshFailed,
  failed,
}

class LibraryRootUpdateTask {
  const LibraryRootUpdateTask({
    required this.root,
    required this.scanId,
    this.phase = LibraryRootUpdatePhase.queued,
    this.visitedEntries = 0,
    this.acceptedItems = 0,
    this.issueCount = 0,
    this.validatedItems = 0,
    this.validationItemCount = 0,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final LibraryRoot root;
  final String scanId;
  final LibraryRootUpdatePhase phase;
  final int visitedEntries;
  final int acceptedItems;
  final int issueCount;
  final int validatedItems;
  final int validationItemCount;
  final String? errorMessage;

  bool get isActive => switch (phase) {
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing ||
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling => true,
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled ||
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => false,
  };

  bool get canCancel => switch (phase) {
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing => true,
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling ||
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled ||
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => false,
  };

  bool get canRetry => switch (phase) {
    LibraryRootUpdatePhase.stale ||
    LibraryRootUpdatePhase.refreshFailed ||
    LibraryRootUpdatePhase.failed => true,
    LibraryRootUpdatePhase.queued ||
    LibraryRootUpdatePhase.discovering ||
    LibraryRootUpdatePhase.finalizing ||
    LibraryRootUpdatePhase.refreshing ||
    LibraryRootUpdatePhase.cancelling ||
    LibraryRootUpdatePhase.completed ||
    LibraryRootUpdatePhase.cancelled => false,
  };

  LibraryRootUpdateTask copyWith({
    LibraryRootUpdatePhase? phase,
    int? visitedEntries,
    int? acceptedItems,
    int? issueCount,
    int? validatedItems,
    int? validationItemCount,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryRootUpdateTask(
      root: root,
      scanId: scanId,
      phase: phase ?? this.phase,
      visitedEntries: visitedEntries ?? this.visitedEntries,
      acceptedItems: acceptedItems ?? this.acceptedItems,
      issueCount: issueCount ?? this.issueCount,
      validatedItems: validatedItems ?? this.validatedItems,
      validationItemCount: validationItemCount ?? this.validationItemCount,
      errorMessage: errorMessage == _unchanged
          ? this.errorMessage
          : errorMessage as String?,
    );
  }

  LibraryRootUpdateTask? applyScanUpdate(LibraryScanUpdate update) {
    return switch (update) {
      LibraryScanStarted() => copyWith(
        phase: LibraryRootUpdatePhase.discovering,
      ),
      LibraryScanProgress(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        copyWith(
          phase: LibraryRootUpdatePhase.discovering,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        ),
      LibraryScanFinalizing(
        :final validatedItems,
        :final totalItems,
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        copyWith(
          phase: LibraryRootUpdatePhase.finalizing,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
          validatedItems: validatedItems,
          validationItemCount: totalItems,
        ),
      LibraryScanCompleted(:final assetCount, :final issueCount) => copyWith(
        phase: LibraryRootUpdatePhase.refreshing,
        acceptedItems: assetCount,
        issueCount: issueCount,
      ),
      LibraryScanCancelled(:final acceptedItems, :final issueCount) => copyWith(
        phase: LibraryRootUpdatePhase.cancelled,
        acceptedItems: acceptedItems,
        issueCount: issueCount,
      ),
      LibraryScanPaused(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        copyWith(
          phase: LibraryRootUpdatePhase.failed,
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
          errorMessage: "图库更新已暂停，请重新开始更新。",
        ),
      LibraryScanStale(:final acceptedItems, :final issueCount) => copyWith(
        phase: LibraryRootUpdatePhase.stale,
        acceptedItems: acceptedItems,
        issueCount: issueCount,
        errorMessage: "更新期间源文件发生变化，请重试。",
      ),
      LibraryScanFailed(:final code, :final message) => copyWith(
        phase: LibraryRootUpdatePhase.failed,
        errorMessage: "$code: $message",
      ),
      LibraryAssetDiscovered() => null,
      LibraryIssueDiscovered() => copyWith(issueCount: issueCount + 1),
    };
  }
}
