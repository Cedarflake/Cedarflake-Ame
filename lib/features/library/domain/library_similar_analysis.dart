enum LibrarySimilarAnalysisPhase { running, completed, cancelled, failed }

class LibrarySimilarAnalysisProgress {
  const LibrarySimilarAnalysisProgress({
    required this.runId,
    required this.phase,
    this.checkedSources = 0,
    this.analyzedContents = 0,
    this.skippedSources = 0,
    this.currentPath,
    this.issueCode,
  });

  final String runId;
  final LibrarySimilarAnalysisPhase phase;
  final int checkedSources;
  final int analyzedContents;
  final int skippedSources;
  final String? currentPath;
  final String? issueCode;
}
