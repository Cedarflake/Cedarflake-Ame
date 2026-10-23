import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../library_duplicate_strings.dart";

enum LibraryResultsProgress {
  ready,
  analyzing,
  waiting,
  paused,
  analysisFailed;

  static LibraryResultsProgress resolveAnalysis({
    required LibraryExactAnalysisController analysis,
    required LibraryExactAnalysisScheduler? scheduler,
    bool hasPendingMembers = false,
  }) {
    if (analysis.isRunning) return analyzing;
    if (analysis.hasFailure || (analysis.progress?.skippedFiles ?? 0) > 0) {
      return analysisFailed;
    }
    if (hasPendingMembers || (scheduler?.hasPendingChanges ?? false)) {
      return scheduler?.isPaused == true ? paused : waiting;
    }
    return ready;
  }

  String? get label => switch (this) {
    ready => null,
    analyzing => LibraryDuplicateStrings.running,
    waiting => LibraryDuplicateStrings.automaticWaiting,
    paused => LibraryDuplicateStrings.paused,
    analysisFailed => LibraryDuplicateStrings.failed,
  };
}
