import "package:flutter/foundation.dart";
import "package:flutter/widgets.dart";

import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_publication.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../library_duplicate_strings.dart";
import "../library_strings.dart";

enum LibraryResultsProgress {
  ready,
  analyzing,
  waiting,
  paused,
  publishing,
  analysisFailed,
  publicationFailed;

  static LibraryResultsProgress resolve({
    required LibraryExactAnalysisController analysis,
    required LibraryExactAnalysisPublication publication,
    required LibraryExactAnalysisScheduler? scheduler,
    required BigInt? displayedRevision,
    required BigInt expectedRevision,
  }) {
    if (analysis.isRunning) {
      return analyzing;
    }
    if (publication.isPublishing) {
      return publishing;
    }
    if (publication.hasFailure) {
      return publicationFailed;
    }
    if (analysis.hasFailure || (analysis.progress?.skippedFiles ?? 0) > 0) {
      return analysisFailed;
    }
    if (scheduler?.hasPendingChanges ?? false) {
      return scheduler?.isPaused == true ? paused : waiting;
    }
    if (displayedRevision == null || displayedRevision < expectedRevision) {
      return publishing;
    }
    return ready;
  }

  String? get label => switch (this) {
    ready => null,
    analyzing => LibraryDuplicateStrings.running,
    waiting => LibraryDuplicateStrings.automaticWaiting,
    paused => LibraryDuplicateStrings.paused,
    publishing => LibraryStrings.refreshingSourceDisplay,
    analysisFailed => LibraryDuplicateStrings.failed,
    publicationFailed => LibraryStrings.synchronizationRefreshFailureTitle,
  };
}

/// Directory synchronization and gallery result publication finish independently.
class LibraryResultsProgressScope extends InheritedWidget {
  const LibraryResultsProgressScope({
    required this.progress,
    required this.rootIds,
    required super.child,
    super.key,
  });

  final LibraryResultsProgress progress;
  final Set<String> rootIds;

  static LibraryResultsProgress forRoot(BuildContext context, String rootId) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<LibraryResultsProgressScope>();
    return scope != null && scope.rootIds.contains(rootId)
        ? scope.progress
        : LibraryResultsProgress.ready;
  }

  @override
  bool updateShouldNotify(LibraryResultsProgressScope oldWidget) =>
      progress != oldWidget.progress || !setEquals(rootIds, oldWidget.rootIds);
}
