import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/fixed_query_projection.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  for (final lateFailure in [false, true]) {
    test(
      "date navigation retires the held synchronization transition ($lateFailure)",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        final synchronization = fixture.viewport.refreshFromSynchronization(
          catalogRevision: BigInt.two,
        );
        expect(fixture.catalog.reads, hasLength(1));
        expect(
          await fixture.viewport.jumpToTime(
            fixture.state.timeline!.buckets.single,
            itemOffset: 60000,
          ),
          isTrue,
        );
        expect(fixture.state.windowStartItemOffset, 60000);
        expect(fixture.state.isLoadingTimeline, isFalse);
        fixture.catalog.finishQuery(0, fails: lateFailure);
        expect(await synchronization, LibraryQueryUpdateOutcome.superseded);
        expect(fixture.state.windowStartItemOffset, 60000);
        fixture.viewport.cancelTimeNavigation();
        fixture.attachPosition(60000);
        fixture.viewport.ensureVisibleRange(
          startItemOffset: 61000,
          endItemOffsetExclusive: 61001,
        );
        await Future<void>.delayed(Duration.zero);
        expect(fixture.catalog.reads, hasLength(2));
        fixture.catalog.completeQuery(1);
        await Future<void>.delayed(Duration.zero);
        expect(fixture.state.isLoadingTimeAnchor, isFalse);
        expect(fixture.state.timeNavigationErrorMessage, isNull);
      },
    );
  }

  test(
    "stale passive time loading refreshes the visible middle window",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final prefetch = fixture.prefetch();
      await _drain();
      fixture.catalog.completeQuery(0);
      expect(await prefetch, isFalse);
      expect(fixture.state.windowStartItemOffset, 52500);
      expect(fixture.state.assets.single.locationId, "location-52500");
      expect(fixture.state.catalogRevision, BigInt.two);
      expect(fixture.state.activeTimeAnchor, isNull);
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
      expect(
        fixture.catalog.reads.single.anchor?.requestedLocationId,
        "location-52500",
      );
      expect(fixture.catalog.semanticReads, 0);
    },
  );

  for (final oldReadFails in [false, true]) {
    test(
      "later scrolling retires a stale prefetch refresh ($oldReadFails)",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        final prefetch = fixture.prefetch();
        await _drain();
        fixture.attachPosition(52480);
        fixture.catalog.finishQuery(0, fails: oldReadFails);
        await _drain();
        expect(fixture.catalog.reads, hasLength(2));
        expect(fixture.state.catalogRevision, BigInt.one);
        expect(fixture.state.windowStartItemOffset, 52500);
        expect(fixture.state.isLoadingTimeAnchor, isTrue);
        expect(fixture.state.timeNavigationErrorMessage, isNull);
        fixture.catalog.completeQuery(1);
        expect(await prefetch, isFalse);
        expect(fixture.state.windowStartItemOffset, 52480);
        expect(fixture.state.assets.single.locationId, "location-52480");
        expect(fixture.state.isLoadingTimeAnchor, isFalse);
        expect(fixture.catalog.semanticReads, 0);
      },
    );

    for (final explicitOffset in [53000, 60000]) {
      test(
        "explicit date $explicitOffset retires the prefetch recovery ($oldReadFails)",
        () async {
          final fixture = _Fixture();
          addTearDown(fixture.dispose);
          final prefetch = fixture.prefetch();
          await _drain();
          final resolution = Completer<LibraryTimeSnapshot>();
          fixture.catalog.pendingResolution = resolution;
          final navigation = fixture.viewport.jumpToTime(
            fixture.state.timeline!.buckets.single,
            itemOffset: explicitOffset,
          );
          fixture.catalog.finishQuery(0, fails: oldReadFails);
          await _drain();
          expect(fixture.catalog.semanticReads, 1);
          expect(fixture.catalog.reads, hasLength(1));
          expect(fixture.state.windowStartItemOffset, 52500);
          expect(fixture.state.catalogRevision, BigInt.one);
          expect(fixture.state.isLoadingTimeAnchor, isTrue);
          expect(fixture.state.timeNavigationErrorMessage, isNull);
          resolution.complete(fixture.catalog.timeSnapshot(explicitOffset));
          expect(await navigation, isTrue);
          expect(await prefetch, explicitOffset == 53000);
          expect(fixture.state.windowStartItemOffset, explicitOffset);
          expect(fixture.state.activeTimeAnchor!.itemOffset, explicitOffset);
          expect(fixture.state.isLoadingTimeAnchor, isFalse);
        },
      );
    }
  }

  for (final retirement in ["cancel", "supersede", "dispose"]) {
    for (final oldReadFails in [false, true]) {
      test("$retirement retires a pending recovery ($oldReadFails)", () async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        final prefetch = fixture.prefetch();
        await _drain();
        switch (retirement) {
          case "cancel":
            fixture.viewport.cancelTimeNavigation();
          case "supersede":
            fixture.viewport.supersedeExternalRequests();
          case "dispose":
            fixture.viewport.dispose();
        }
        final retiredState = fixture.state;
        fixture.catalog.finishQuery(0, fails: oldReadFails);
        expect(await prefetch, isFalse);
        await _drain();
        expect(fixture.state, same(retiredState));
        expect(fixture.catalog.reads, hasLength(1));
        expect(fixture.catalog.semanticReads, 0);
      });
    }
  }

  test(
    "a current recovery failure retains the window and reports its error",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final prefetch = fixture.prefetch();
      await _drain();
      fixture.catalog.finishQuery(0, fails: true);
      expect(await prefetch, isFalse);
      expect(fixture.state.windowStartItemOffset, 52500);
      expect(fixture.state.catalogRevision, BigInt.one);
      expect(
        fixture.state.timeNavigationErrorMessage,
        contains("catalog_database_busy"),
      );
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
      expect(fixture.catalog.reads, hasLength(1));
    },
  );
}

Future<void> _drain() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}

class _Fixture {
  _Fixture() {
    scanner.checkpoint = null;
    catalog = _Catalog(scanner);
    state =
        LibraryState.fromSnapshot(
          catalog.page(52500, revision: BigInt.one),
          query: query,
        ).copyWith(
          timeline: catalog.timeline(BigInt.one),
          windowStartItemOffset: 52500,
        );
    viewport = LibraryViewportController(
      catalog,
      () => state,
      (next) => state = next,
      () => false,
      (_) {},
    );
    viewport.seed(state);
    attachPosition(52500);
  }

  static const query = LibraryGalleryQuery(rootId: "other");
  final scanner = RetainedScanScanner();
  late final _Catalog catalog;
  late LibraryState state;
  late final LibraryViewportController viewport;

  void attachPosition(int offset) => viewport.queryProjections.attach(
    FixedQueryProjection(
      LibraryQueryAnchor(
        requestedLocationId: "location-$offset",
        assetId: "asset-$offset",
        fallbackGlobalItemIndex: offset,
      ),
    ),
  );

  Future<bool> prefetch() =>
      viewport.prefetchTime(state.timeline!.buckets.single, itemOffset: 53000);

  void dispose() {
    viewport.dispose();
    scanner.dispose();
  }
}

class _Catalog extends RetainedScanCatalog {
  _Catalog(super.scanner);

  final reads =
      <
        ({LibraryQueryAnchor? anchor, Completer<LibraryQuerySnapshot> result})
      >[];
  int semanticReads = 0;
  Completer<LibraryTimeSnapshot>? pendingResolution;

  LibraryTimeline timeline(BigInt revision) => LibraryTimeline(
    revision: revision,
    queryId: "query-other",
    totalItems: 79281,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2012-03",
        itemCount: 79281,
        aspectRatioSum: 79281,
      ),
    ],
  );

  LibrarySnapshot page(
    int offset, {
    BigInt? revision,
    LibraryQueryAnchor? anchor,
  }) {
    final original = snapshot(_Fixture.query, offset: offset);
    return LibrarySnapshot(
      roots: original.roots,
      assets: original.assets,
      catalogPath: original.catalogPath,
      queryId: original.queryId,
      revision: revision ?? BigInt.two,
      queryAnchorResolution: anchor == null
          ? null
          : LibraryQueryAnchorResolution(
              requestedLocationId: anchor.requestedLocationId,
              locationId: original.assets.single.locationId,
              ordinal: offset,
              windowStartItemOffset: offset,
            ),
    );
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => throw const LibraryCatalogFailure(
    code: "catalog_cursor_stale",
    message: "A source reconciliation replaced the old time cursor",
  );

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) {
    expectSync(query, _Fixture.query);
    final read = (anchor: anchor, result: Completer<LibraryQuerySnapshot>());
    reads.add(read);
    return read.result.future;
  }

  void completeQuery(int index) {
    final read = reads[index];
    read.result.complete(
      LibraryQuerySnapshot(
        snapshot: page(
          read.anchor?.fallbackGlobalItemIndex ?? 0,
          anchor: read.anchor,
        ),
        timeline: timeline(BigInt.two),
      ),
    );
  }

  void finishQuery(int index, {required bool fails}) {
    if (fails) {
      reads[index].result.completeError(
        const LibraryCatalogFailure(
          code: "catalog_database_busy",
          message: "The projected query could not be read",
        ),
      );
    } else {
      completeQuery(index);
    }
  }

  LibraryTimeSnapshot timeSnapshot(int offset) => LibraryTimeSnapshot(
    snapshot: page(offset),
    timeline: timeline(BigInt.two),
    anchor: LibraryTimeAnchor(
      revision: BigInt.two,
      queryId: "query-other",
      monthKey: "2012-03",
      itemOffset: offset,
    ),
    windowStartItemOffset: offset,
  );

  @override
  Future<LibraryTimeSnapshot> resolveTimeIntent({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeIntent intent,
  }) {
    semanticReads++;
    return pendingResolution?.future ??
        Future.value(timeSnapshot(intent.itemOffset));
  }
}
