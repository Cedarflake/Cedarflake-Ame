import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  test("published root browsing continues while a peer import runs", () async {
    final fixture = await _startImport();
    final scan = fixture.state.primaryScanSnapshot;
    const query = LibraryGalleryQuery(rootId: "other");

    expect(await fixture.controller.updateQuery(query), isTrue);
    expect(fixture.state.query, query);
    await fixture.controller.loadNextPage();
    expect(fixture.catalog.afterLoads, 1);
    expect(await fixture.controller.loadPreviousPage(), isTrue);
    expect(fixture.catalog.beforeLoads, 1);
    expect(
      await fixture.controller.jumpToTime(
        fixture.state.timeline!.buckets.single,
        itemOffset: 4,
      ),
      isTrue,
    );
    expect(fixture.catalog.timeLoads, 1);
    expect(fixture.state.windowStartItemOffset, 4);
    expect(fixture.state.primaryScanSnapshot, same(scan));
    expect(fixture.state.isScanning, isTrue);
    expect(fixture.scanner.startedRoots, [r"C:\Incoming"]);
    expect(fixture.scanner.cancelActiveIds, isEmpty);
  });

  test("passive catalog refresh does not wait for a peer import", () async {
    final fixture = await _startImport();
    final scan = fixture.state.primaryScanSnapshot;
    final reads = fixture.catalog.firstLoads;

    expect(
      await fixture.controller.refreshFromSynchronization(
        catalogRevision: BigInt.one,
      ),
      LibraryQueryUpdateOutcome.applied,
    );
    expect(fixture.catalog.firstLoads, reads + 1);
    expect(fixture.state.primaryScanSnapshot, same(scan));
    expect(fixture.state.isScanning, isTrue);
  });

  test("browsing cannot admit a second import or root removal", () async {
    final fixture = await _startImport();
    await fixture.controller.updateQuery(
      const LibraryGalleryQuery(rootId: "other"),
    );
    await fixture.controller.scanDirectory(r"C:\Second import");
    expect(fixture.controller.prepareRootRemoval(otherPublishedRoot), isNull);
    expect(fixture.scanner.startedRoots, [r"C:\Incoming"]);
    expect(fixture.catalog.removedIds, isEmpty);
  });

  test("latest root query wins without cancelling the import", () async {
    final fixture = await _startImport();
    const obsoleteQuery = LibraryGalleryQuery(rootId: "other");
    const latestQuery = LibraryGalleryQuery(searchText: "latest");
    final obsolete = fixture.catalog.pendingLoad = Completer<LibrarySnapshot>();
    final first = fixture.controller.updateQuery(obsoleteQuery);
    await flushRetainedScanMicrotasks();
    expect(await fixture.controller.updateQuery(latestQuery), isTrue);
    obsolete.complete(fixture.catalog.snapshot(obsoleteQuery));
    expect(await first, isFalse);
    expect(fixture.state.query, latestQuery);
    expect(fixture.state.isRefreshingQuery, isFalse);
    expect(fixture.state.isScanning, isTrue);
  });

  test(
    "disposal retires an import-time query before its late result",
    () async {
      final fixture = await _startImport();
      const query = LibraryGalleryQuery(rootId: "other");
      final response = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      final pending = fixture.controller.updateQuery(query);
      await flushRetainedScanMicrotasks();
      fixture.dispose();
      expect(await pending, isFalse);
      response.complete(fixture.catalog.snapshot(query));
      await flushRetainedScanMicrotasks();
    },
  );

  test(
    "query failure preserves an active import and the published gallery",
    () async {
      final fixture = await _startImport();
      final scan = fixture.state.primaryScanSnapshot;
      final assets = fixture.state.assets;
      fixture.catalog.loadFailure = StateError("controlled query failure");

      expect(
        await fixture.controller.updateQuery(
          const LibraryGalleryQuery(rootId: "other"),
        ),
        isFalse,
      );
      expect(fixture.state.queryActivity, isA<LibraryQueryFailed>());
      expect(fixture.state.assets, same(assets));
      expect(fixture.state.primaryScanSnapshot, same(scan));
      expect(fixture.scanner.startedRoots, [r"C:\Incoming"]);
    },
  );

  test(
    "import completion reloads the latest query after its pending read",
    () async {
      final fixture = await _startImport();
      const query = LibraryGalleryQuery(rootId: "other", searchText: "latest");
      final pending = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      final selection = fixture.controller.updateQuery(query);
      await flushRetainedScanMicrotasks();
      expect(fixture.state.queryActivity, isA<LibraryQueryLoading>());
      fixture.scanner.completeScan(fixture.state.scanId!);
      await flushRetainedScanMicrotasks();
      expect(
        fixture.state.primaryScanSnapshot.publication,
        LibraryScanPublication.reloadPending,
      );
      pending.complete(fixture.catalog.snapshot(query));

      expect(await selection, isTrue);
      await flushRetainedScanMicrotasks();
      expect(fixture.state.query, query);
      expect(
        fixture.state.primaryScanSnapshot.publication,
        LibraryScanPublication.visible,
      );
      expect(fixture.state.status, LibraryStatus.completed);
      expect(fixture.scanner.startedRoots, [r"C:\Incoming"]);
    },
  );
}

Future<RetainedScanFixture> _startImport() async {
  final fixture = RetainedScanFixture();
  fixture.scanner.checkpoint = null;
  await fixture.restore();
  await fixture.controller.scanDirectory(r"C:\Incoming");
  expect(fixture.state.isScanning, isTrue);
  return fixture;
}
