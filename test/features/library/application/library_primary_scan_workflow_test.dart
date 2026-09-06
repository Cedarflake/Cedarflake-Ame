import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scan_shutdown.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  test(
    "restoration survives a concurrent gallery query without executing",
    () async {
      final fixture = RetainedScanFixture();
      final checkpoint = fixture.scanner.pendingRecovery = Completer();
      final load = fixture.catalog.pendingLoad = Completer();
      final controller = fixture.controller;
      await flushRetainedScanMicrotasks();
      final query = controller.updateQuery(
        const LibraryGalleryQuery(rootId: "other"),
      );
      await flushRetainedScanMicrotasks();
      checkpoint.complete(retainedScanCheckpoint);
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.state.isRefreshingQuery, isTrue);
      load.complete(
        fixture.catalog.snapshot(const LibraryGalleryQuery(rootId: "other")),
      );
      expect(await query, isTrue);
      expect(fixture.state.query.rootId, "other");
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  test(
    "paused task survives query, paging, time navigation and sync refresh",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      final task = fixture.state.primaryScan;
      expect(fixture.state.isBusy, isFalse);
      expect(
        await fixture.controller.updateQuery(
          const LibraryGalleryQuery(rootId: "other"),
        ),
        isTrue,
      );
      await fixture.controller.loadNextPage();
      expect(fixture.catalog.afterLoads, 1);
      expect(await fixture.controller.loadPreviousPage(), isTrue);
      expect(fixture.catalog.beforeLoads, 1);
      final bucket = fixture.state.timeline!.buckets.single;
      expect(
        await fixture.controller.jumpToTime(bucket, itemOffset: 3),
        isTrue,
      );
      expect(fixture.catalog.timeLoads, 1);
      expect(
        await fixture.controller.refreshFromSynchronization(
          catalogRevision: BigInt.one,
        ),
        LibraryQueryUpdateOutcome.applied,
      );
      expect(fixture.state.primaryScan, same(task));
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.state.stagedAssetCount, 40);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  test(
    "durable cancel waits for commit and a late query cannot revive it",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      final load = fixture.catalog.pendingLoad = Completer();
      final query = fixture.controller.updateQuery(
        const LibraryGalleryQuery(rootId: "other"),
      );
      await flushRetainedScanMicrotasks();
      final commit = fixture.scanner.pendingCancellation = Completer();
      final cancellation = fixture.controller.cancelScan();
      final duplicate = fixture.controller.cancelScan();
      expect(fixture.state.status, LibraryStatus.discarding);
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      expect(fixture.scanner.cancelRetainedIds, [
        retainedScanCheckpoint.scanId,
      ]);
      await fixture.controller.resumePausedScan();
      expect(fixture.scanner.startedRoots, isEmpty);
      commit.complete();
      await cancellation;
      await duplicate;
      expect(fixture.state.hasRetainedScan, isFalse);
      load.complete(
        fixture.catalog.snapshot(const LibraryGalleryQuery(rootId: "other")),
      );
      expect(await query, isTrue);
      expect(fixture.state.scanId, isNull);
      expect(fixture.state.query.rootId, "other");
      await fixture.controller.chooseDirectoryAndScan();
      expect(fixture.picker.calls, 1);
      expect(fixture.scanner.startedRoots, ["C:\\New"]);
    },
  );

  test(
    "failed retained cancellation preserves intent and can be retried",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      fixture.scanner.cancelFailure = StateError("write busy");
      await fixture.controller.cancelScan();
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.state.errorMessage, contains("write busy"));
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      await fixture.controller.chooseDirectoryAndScan();
      expect(fixture.picker.calls, 0);
      fixture.scanner.cancelFailure = null;
      await fixture.controller.cancelScan();
      expect(fixture.state.scanId, isNull);
      expect(fixture.state.errorMessage, isNull);
      expect(fixture.scanner.cancelRetainedIds, [
        retainedScanCheckpoint.scanId,
        retainedScanCheckpoint.scanId,
      ]);
      expect(fixture.scanner.cancelActiveIds, isEmpty);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  test(
    "other-root removal and updates cannot consume a paused first import",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      final task = fixture.state.primaryScan;
      final updates = fixture.container.read(
        libraryUpdateControllerProvider.notifier,
      );
      updates.startUpdates(["retained", "other"]);
      await flushRetainedScanMicrotasks();
      expect(fixture.scanner.startedRoots, ["C:\\Other"]);
      updates.cancel("other");
      fixture.scanner.finishUpdates();
      await flushRetainedScanMicrotasks();
      expect(
        await fixture.controller.unregisterRoot(otherPublishedRoot),
        isTrue,
      );
      expect(fixture.state.primaryScan, same(task));
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.scanner.cancelRetainedIds, isEmpty);
    },
  );

  for (final sameRoot in [false, true]) {
    test(
      "${sameRoot ? 'same' : 'other'} root removal retires intent at commit despite reload failure",
      () async {
        final fixture = RetainedScanFixture();
        await fixture.restore();
        fixture.catalog.failAfterRemoval = true;
        expect(
          await fixture.controller.unregisterRoot(
            sameRoot ? retainedScanRoot : otherPublishedRoot,
          ),
          isFalse,
        );
        expect(fixture.state.isRemovalCommitted, isTrue);
        expect(fixture.state.hasRetainedScan, !sameRoot);
        expect(
          fixture.state.primaryScan!.scanId,
          sameRoot ? isNull : retainedScanCheckpoint.scanId,
        );
        fixture.catalog.failAfterRemoval = false;
        await fixture.controller.retry();
        expect(fixture.catalog.removedIds, [sameRoot ? "retained" : "other"]);
        expect(fixture.state.isRemovalCommitted, isFalse);
        expect(fixture.state.hasRetainedScan, !sameRoot);
        expect(fixture.scanner.startedRoots, isEmpty);
      },
    );
  }

  test(
    "Continue supersedes an in-flight query without accepting its late state",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      final load = fixture.catalog.pendingLoad = Completer();
      final query = fixture.controller.updateQuery(
        const LibraryGalleryQuery(rootId: "other"),
      );
      await flushRetainedScanMicrotasks();
      await fixture.controller.resumePausedScan();
      expect(fixture.scanner.resumedIds, [retainedScanCheckpoint.scanId]);
      expect(fixture.state.status, LibraryStatus.scanning);
      expect(fixture.state.isRefreshingQuery, isFalse);
      load.complete(
        fixture.catalog.snapshot(const LibraryGalleryQuery(rootId: "other")),
      );
      expect(await query, isFalse);
      expect(fixture.state.status, LibraryStatus.scanning);
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
    },
  );

  test(
    "failed startup lookup retries lookup, never implicitly starts a scan",
    () async {
      final fixture = RetainedScanFixture();
      fixture.scanner.recoveryFailure = StateError("checkpoint read failed");
      await fixture.restore();
      expect(fixture.state.status, LibraryStatus.failed);
      fixture.scanner.recoveryFailure = null;
      await fixture.controller.retry();
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.scanner.startedRoots, isEmpty);
      expect(fixture.picker.calls, 0);
    },
  );

  test("published scan retries only its failed display reload", () async {
    final fixture = RetainedScanFixture();
    await fixture.restore();
    await fixture.controller.cancelScan();
    await fixture.controller.scanDirectory("C:\\Other");
    final scanId = fixture.state.scanId!;
    fixture.catalog.loadFailure = StateError("published display read failed");
    fixture.scanner.completeScan(scanId);
    await flushRetainedScanMicrotasks();
    expect(fixture.state.status, LibraryStatus.failed);
    expect(
      fixture.state.primaryScan!.publication,
      LibraryScanPublication.reloadPending,
    );
    await fixture.controller.retry();
    expect(fixture.state.status, LibraryStatus.failed);
    expect(fixture.scanner.startedRoots, ["C:\\Other"]);
    fixture.catalog.loadFailure = null;
    await fixture.controller.retry();
    expect(fixture.state.status, LibraryStatus.completed);
    expect(
      fixture.state.primaryScan!.publication,
      LibraryScanPublication.visible,
    );
    expect(fixture.scanner.startedRoots, ["C:\\Other"]);
    expect(fixture.scanner.resumedIds, isEmpty);
  });

  test(
    "superseded publication reload cannot report completion or repeat a scan",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      await fixture.controller.cancelScan();
      await fixture.controller.scanDirectory("C:\\Other");
      final scanId = fixture.state.scanId!;
      final reload = fixture.catalog.pendingLoad = Completer();
      fixture.scanner.completeScan(scanId);
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.refreshing);
      expect(
        await fixture.controller.updateQuery(
          const LibraryGalleryQuery(rootId: "retained"),
        ),
        isTrue,
      );
      reload.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.failed);
      expect(
        fixture.state.errorMessage,
        contains("catalog_publication_view_superseded"),
      );
      expect(
        fixture.state.primaryScan!.publication,
        LibraryScanPublication.reloadPending,
      );
      await fixture.controller.retry();
      expect(fixture.state.status, LibraryStatus.completed);
      expect(fixture.state.query.rootId, "retained");
      expect(fixture.scanner.startedRoots, ["C:\\Other"]);
      expect(fixture.scanner.resumedIds, isEmpty);
    },
  );

  for (final timeline in [false, true]) {
    test(
      "paused intent survives ${timeline ? 'timeline' : 'catalog'} query failure and read-only retry",
      () async {
        final fixture = RetainedScanFixture();
        await fixture.restore();
        final task = fixture.state.primaryScan;
        final failure = StateError("requested query failed");
        if (timeline) {
          fixture.catalog.timelineFailure = failure;
        } else {
          fixture.catalog.loadFailure = failure;
        }
        expect(
          await fixture.controller.updateQuery(
            const LibraryGalleryQuery(rootId: "other"),
          ),
          isFalse,
        );
        final queryFailure = fixture.state.queryActivity as LibraryQueryFailed;
        expect(queryFailure.requestedQuery.rootId, "other");
        expect(queryFailure.message, contains("requested query failed"));
        expect(fixture.state.primaryScan, same(task));
        expect(fixture.state.query.rootId, isNull);
        expect(fixture.state.errorMessage, isNull);
        fixture.catalog.loadFailure = null;
        fixture.catalog.timelineFailure = null;
        expect(
          await fixture.controller.updateQuery(
            queryFailure.requestedQuery,
            forceRefresh: true,
          ),
          isTrue,
        );
        expect(fixture.state.queryActivity, isA<LibraryQueryIdle>());
        expect(fixture.state.primaryScan, same(task));
        expect(fixture.scanner.startedRoots, isEmpty);
        expect(fixture.scanner.resumedIds, isEmpty);
      },
    );
  }

  test("close waits for an admitted durable cancel without resuming", () async {
    final fixture = RetainedScanFixture();
    await fixture.restore();
    final commit = fixture.scanner.pendingCancellation = Completer();
    final cancellation = fixture.controller.cancelScan();
    var didClose = false;
    final close = fixture.container
        .read(libraryScanShutdownCoordinatorProvider)
        .suspend()
        .then((_) => didClose = true);
    await flushRetainedScanMicrotasks();
    expect(didClose, isFalse);
    commit.complete();
    await cancellation;
    await close;
    expect(didClose, isTrue);
    expect(fixture.scanner.startedRoots, isEmpty);
  });

  test(
    "dispose cannot revoke a committed cancel or publish to a dead notifier",
    () async {
      final fixture = RetainedScanFixture();
      await fixture.restore();
      final commit = fixture.scanner.pendingCancellation = Completer();
      final cancellation = fixture.controller.cancelScan();
      fixture.dispose();
      commit.complete();
      await cancellation;
      expect(fixture.scanner.checkpoint, isNull);
      expect(fixture.scanner.cancelActiveIds, isEmpty);
    },
  );
}
