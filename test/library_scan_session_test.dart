import "package:cedarflake_ame/features/library/application/library_scan_session.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("tracks recoverable scan progress and pauses with current counters", () {
    final session = LibraryScanSession();
    session.begin(_scan());

    final progress = session.apply(
      const LibraryState(status: LibraryStatus.scanning),
      const LibraryScanProgress(
        visitedEntries: 30,
        acceptedItems: 12,
        issueCount: 2,
      ),
    );
    final paused = session.apply(
      progress.state,
      const LibraryScanPaused(
        visitedEntries: 31,
        acceptedItems: 13,
        issueCount: 3,
      ),
    );

    expect(paused.state.status, LibraryStatus.paused);
    expect(paused.state.visitedEntries, 31);
    expect(paused.state.stagedAssetCount, 13);
    expect(session.activeScanId, isNull);
    expect(session.pausedScan?.visitedEntries, 31);
    expect(session.pausedScan?.acceptedItems, 13);
    expect(session.pausedScan?.issueCount, 3);
  });

  test("bounds recent issues without losing the total issue count", () {
    final session = LibraryScanSession();
    var state = const LibraryState(status: LibraryStatus.scanning);

    for (var index = 0; index < 24; index += 1) {
      state = session
          .apply(
            state,
            LibraryIssueDiscovered(
              LibraryIssue(code: "issue-$index", message: "Issue $index"),
            ),
          )
          .state;
    }

    expect(state.issueCount, 24);
    expect(state.recentIssues, hasLength(20));
    expect(state.recentIssues.first.code, "issue-4");
    expect(state.recentIssues.last.code, "issue-23");
  });

  test("publishes completed scans through a catalog reload transition", () {
    final session = LibraryScanSession();
    session.begin(_scan());

    final transition = session.apply(
      const LibraryState(status: LibraryStatus.scanning, isResumingScan: true),
      const LibraryScanCompleted(
        assetCount: 10,
        issueCount: 1,
        catalogPath: "C:\\Ame\\catalog.sqlite3",
        wasLimited: false,
      ),
    );

    expect(transition.shouldReloadCatalog, isTrue);
    expect(transition.state.status, LibraryStatus.refreshing);
    expect(transition.state.catalogPath, "C:\\Ame\\catalog.sqlite3");
    expect(transition.state.isResumingScan, isFalse);
    expect(session.activeScanId, isNull);
  });

  test("marks an active scan failed when its stream ends unexpectedly", () {
    final session = LibraryScanSession();
    session.begin(_scan());

    final state = session.finish(
      const LibraryState(status: LibraryStatus.scanning),
    );

    expect(state.status, LibraryStatus.failed);
    expect(state.errorMessage, "The scan ended without a completion event");
    expect(session.activeScanId, isNull);
  });
}

RecoverableLibraryScan _scan() {
  return const RecoverableLibraryScan(
    scanId: "scan-1",
    rootPath: "C:\\Pictures",
    previewEdge: 512,
    visitedEntries: 0,
    acceptedItems: 0,
    issueCount: 0,
  );
}
