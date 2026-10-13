import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final retry in [false, true]) {
    test(
      "passive synchronization cannot supersede ${retry ? 'retried' : 'initial'} primary publication",
      () async {
        final fixture = await _prepareScan();
        final scanId = fixture.state.scanId!;
        if (retry) {
          fixture.catalog.loadFailure = StateError(
            "committed display read failed",
          );
          fixture.scanner.completeScan(scanId);
          await flushRetainedScanMicrotasks();
          expect(fixture.state.status, LibraryStatus.failed);
          fixture.catalog.loadFailure = null;
        }
        final readsBefore = fixture.catalog.firstLoads;
        final result = fixture.catalog.pendingLoad =
            Completer<LibrarySnapshot>();
        final Future<void> task;
        if (retry) {
          task = fixture.controller.retry();
        } else {
          fixture.scanner.completeScan(scanId);
          task = Future<void>.value();
        }
        await flushRetainedScanMicrotasks();
        final duringRead = fixture.state.primaryScanSnapshot;
        final passive = await fixture.controller.refreshFromSynchronization(
          catalogRevision: BigInt.one,
        );
        result.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
        await task;
        await flushRetainedScanMicrotasks();

        expect(duringRead.status, LibraryStatus.refreshing);
        expect(duringRead.publication, LibraryScanPublication.reloadPending);
        expect(passive, LibraryQueryUpdateOutcome.busy);
        expect(fixture.catalog.firstLoads, readsBefore + 1);
        expect(fixture.state.status, LibraryStatus.completed);
        expect(
          fixture.state.primaryScanSnapshot.publication,
          LibraryScanPublication.visible,
        );
        expect(fixture.scanner.startedRoots, [r"C:\Other"]);
        expect(fixture.scanner.resumedIds, isEmpty);
      },
    );
  }

  test(
    "primary retry waits for an active passive read without overlapping it",
    () async {
      final fixture = await _prepareScan();
      fixture.catalog.loadFailure = StateError("committed display read failed");
      fixture.scanner.completeScan(fixture.state.scanId!);
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.failed);
      fixture.catalog.loadFailure = null;

      final readsBefore = fixture.catalog.firstLoads;
      final pending = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      final passive = fixture.controller.refreshFromSynchronization(
        catalogRevision: BigInt.one,
      );
      await flushRetainedScanMicrotasks();
      final retry = fixture.controller.retry();
      await flushRetainedScanMicrotasks();
      final readsWhileHeld = fixture.catalog.firstLoads;
      final competing = await fixture.controller.refreshFromSynchronization(
        catalogRevision: BigInt.one,
      );
      pending.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
      final passiveOutcome = await passive;
      await retry;

      expect(readsWhileHeld, readsBefore + 1);
      expect(competing, LibraryQueryUpdateOutcome.busy);
      expect(passiveOutcome, LibraryQueryUpdateOutcome.applied);
      expect(fixture.catalog.firstLoads, readsBefore + 2);
      expect(fixture.state.status, LibraryStatus.completed);
      expect(
        fixture.state.primaryScanSnapshot.publication,
        LibraryScanPublication.visible,
      );
      expect(fixture.scanner.startedRoots, [r"C:\Other"]);
      expect(fixture.scanner.resumedIds, isEmpty);
    },
  );
}

Future<RetainedScanFixture> _prepareScan() async {
  final fixture = RetainedScanFixture();
  await fixture.restore();
  await fixture.controller.cancelScan();
  await fixture.controller.scanDirectory(r"C:\Other");
  return fixture;
}
