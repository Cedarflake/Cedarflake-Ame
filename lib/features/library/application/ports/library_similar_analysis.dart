import "../../domain/library_similar_analysis.dart";

abstract interface class LibrarySimilarAnalysis {
  Future<LibrarySimilarAnalysisSession> open(String runId);
}

abstract interface class LibrarySimilarAnalysisSession {
  Future<LibrarySimilarAnalysisProgress> run();
  LibrarySimilarAnalysisProgress progress();
  void cancel();
  void suspend(bool suspended);
  void dispose();
}
