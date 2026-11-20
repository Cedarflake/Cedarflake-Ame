import "../../../../app/presentation/ame_localizations.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";

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

  String? label(AppLocalizations strings) => switch (this) {
    ready => null,
    analyzing => strings.duplicateRunning,
    waiting => strings.waitingForBackground,
    paused => strings.paused,
    analysisFailed => strings.duplicateIncomplete,
  };
}
