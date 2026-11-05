import "../../../../src/rust/api/similar_images.dart" as rust_api;
import "../../../../src/rust/domain/similar_images/analysis.dart" as rust;
import "../../application/ports/library_similar_analysis.dart";
import "../../domain/library_similar_analysis.dart";
import "library_duplicate_failure.dart";

class RustLibrarySimilarAnalysis implements LibrarySimilarAnalysis {
  const RustLibrarySimilarAnalysis();

  @override
  Future<LibrarySimilarAnalysisSession> open(String runId) async => _Session(
    await invokeDuplicateBridge(
      () => rust_api.openSimilarImageAnalysis(runId: runId),
    ),
  );
}

class _Session implements LibrarySimilarAnalysisSession {
  _Session(this._native);
  final rust_api.SimilarImageAnalysis _native;

  @override
  Future<LibrarySimilarAnalysisProgress> run() async =>
      _map(await invokeDuplicateBridge(_native.run));

  @override
  LibrarySimilarAnalysisProgress progress() =>
      invokeDuplicateSync(() => _map(_native.progress()));

  @override
  void cancel() => invokeDuplicateSync(_native.cancel);

  @override
  void suspend(bool suspended) =>
      invokeDuplicateSync(() => _native.suspend(suspended: suspended));

  @override
  void dispose() => invokeDuplicateSync(_native.dispose);
}

LibrarySimilarAnalysisProgress _map(rust.SimilarAnalysisProgress progress) =>
    LibrarySimilarAnalysisProgress(
      runId: progress.runId,
      phase: switch (progress.phase) {
        rust.SimilarAnalysisPhase.running =>
          LibrarySimilarAnalysisPhase.running,
        rust.SimilarAnalysisPhase.completed =>
          LibrarySimilarAnalysisPhase.completed,
        rust.SimilarAnalysisPhase.cancelled =>
          LibrarySimilarAnalysisPhase.cancelled,
        rust.SimilarAnalysisPhase.failed => LibrarySimilarAnalysisPhase.failed,
      },
      checkedSources: progress.checkedSources.toInt(),
      analyzedContents: progress.analyzedContents.toInt(),
      skippedSources: progress.skippedSources.toInt(),
      currentPath: progress.currentPath,
      issueCode: progress.issueCode,
    );
