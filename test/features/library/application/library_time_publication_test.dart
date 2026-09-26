import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final isEmpty in [false, true]) {
    test(
      "removed month resolves to ${isEmpty ? 'empty results' : 'a surviving date'}",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        final pending = Completer<LibraryTimeSnapshot>();
        fixture.catalog.pendingResolution = pending;
        final result = fixture.viewport.jumpToTime(
          fixture.historical,
          itemOffset: 10,
        );
        await _drain();
        final base = fixture.catalog.snapshot(_Fixture.query);
        pending.complete(
          LibraryTimeSnapshot(
            snapshot: LibrarySnapshot(
              catalogPath: base.catalogPath,
              revision: BigInt.two,
              queryId: base.queryId,
              roots: base.roots,
              assets: isEmpty ? const [] : base.assets,
            ),
            timeline: LibraryTimeline(
              revision: BigInt.two,
              queryId: base.queryId,
              totalItems: isEmpty ? 0 : 1,
              buckets: isEmpty
                  ? const []
                  : const [
                      LibraryTimeBucket(
                        monthKey: "2010-01",
                        itemCount: 1,
                        aspectRatioSum: 1,
                      ),
                    ],
            ),
            anchor: isEmpty
                ? null
                : LibraryTimeAnchor(
                    revision: BigInt.two,
                    queryId: base.queryId,
                    monthKey: "2010-01",
                    itemOffset: 0,
                  ),
            windowStartItemOffset: 0,
          ),
        );
        expect(await result, isTrue);
        expect(fixture.state.timeline!.totalItems, isEmpty ? 0 : 1);
        expect(
          fixture.state.activeTimeAnchor?.monthKey,
          isEmpty ? null : "2010-01",
        );
        expect(fixture.state.isLoadingTimeAnchor, isFalse);
        expect(fixture.catalog.firstLoads, 0);
      },
    );
  }

  test(
    "a page excluding its resolved date cannot replace the gallery",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final pending = Completer<LibraryTimeSnapshot>();
      fixture.catalog.pendingResolution = pending;
      final result = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      final resolved = fixture.catalog.resolution(
        fixture.catalog.intents.single,
      );
      pending.complete(
        LibraryTimeSnapshot(
          snapshot: resolved.snapshot,
          timeline: resolved.timeline,
          anchor: resolved.anchor,
          windowStartItemOffset: 31,
        ),
      );
      expect(await result, isFalse);
      expect(fixture.state.catalogRevision, BigInt.one);
      expect(
        fixture.state.timeNavigationErrorMessage,
        contains("catalog_time_snapshot_invalid"),
      );
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
      expect(fixture.catalog.firstLoads, 0);
    },
  );

  test(
    "a centered page retains the semantic date separately from its start",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final pending = Completer<LibraryTimeSnapshot>();
      fixture.catalog.pendingResolution = pending;
      final result = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      final base = fixture.catalog.snapshot(_Fixture.query);
      pending.complete(
        LibraryTimeSnapshot(
          snapshot: LibrarySnapshot(
            catalogPath: base.catalogPath,
            revision: base.revision,
            queryId: base.queryId,
            roots: base.roots,
            assets: [
              for (var index = 20; index < 40; index++)
                ...fixture.catalog
                    .snapshot(_Fixture.query, offset: index)
                    .assets,
            ],
          ),
          timeline: fixture.catalog.timeline(BigInt.two),
          anchor: LibraryTimeAnchor(
            revision: BigInt.two,
            queryId: "query-other",
            monthKey: "2012-03",
            itemOffset: 10,
          ),
          windowStartItemOffset: 20,
        ),
      );
      expect(await result, isTrue);
      expect(fixture.state.windowStartItemOffset, 20);
      expect(fixture.state.assets[10].locationId, "location-30");
      expect(fixture.state.activeTimeAnchor!.itemOffset, 10);
      expect(fixture.catalog.firstLoads, 0);
    },
  );

  test(
    "a deletion publication retains the explicit historical target",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final accepted = await fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      expect(accepted, isTrue);
      expect(fixture.catalog.strictReads, 1);
      expect(fixture.catalog.intents, hasLength(1));
      expect(fixture.state.catalogRevision, BigInt.two);
      expect(fixture.state.timeline!.totalItems, 50);
      expect(fixture.state.windowStartItemOffset, 30);
      expect(fixture.state.activeTimeAnchor!.monthKey, "2012-03");
      expect(fixture.state.activeTimeAnchor!.itemOffset, 10);
      expect(fixture.state.assets.single.locationId, "location-30");
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
      expect(fixture.state.timeNavigationErrorMessage, isNull);
      expect(fixture.catalog.firstLoads, 0);
    },
  );

  for (final terminal in ["cancel", "dispose", "supersede"]) {
    test("$terminal retires a pending date resolution", () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final pending = Completer<LibraryTimeSnapshot>();
      fixture.catalog.pendingResolution = pending;
      final result = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      expect(fixture.catalog.intents, hasLength(1));
      switch (terminal) {
        case "cancel":
          fixture.viewport.cancelTimeNavigation();
        case "dispose":
          fixture.viewport.dispose();
        case "supersede":
          fixture.viewport.supersedeExternalRequests();
      }
      final terminalState = fixture.state;
      pending.complete(
        fixture.catalog.resolution(fixture.catalog.intents.single),
      );
      expect(await result, isFalse);
      await _drain();
      expect(fixture.state, same(terminalState));
      expect(fixture.state.catalogRevision, BigInt.one);
      expect(fixture.state.activeTimeAnchor, isNull);
    });
  }

  test(
    "cancellation before a stale strict read admits no semantic read",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final strict = Completer<LibrarySnapshot>();
      fixture.catalog.pendingStrict = strict;
      final result = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      fixture.viewport.cancelTimeNavigation();
      strict.completeError(_PublishingCatalog.stale);
      expect(await result, isFalse);
      expect(fixture.catalog.intents, isEmpty);
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
    },
  );

  test("passive prefetch never gains explicit date authority", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    expect(
      await fixture.viewport.prefetchTime(fixture.historical, itemOffset: 10),
      isFalse,
    );
    expect(fixture.catalog.intents, isEmpty);
    expect(fixture.catalog.firstLoads, 1);
    expect(fixture.state.activeTimeAnchor, isNull);
  });

  test(
    "a pending prefetch may be promoted by the same explicit target",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final strict = Completer<LibrarySnapshot>();
      fixture.catalog.pendingStrict = strict;
      final prefetch = fixture.viewport.prefetchTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      final explicit = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      strict.completeError(_PublishingCatalog.stale);
      expect(await explicit, isTrue);
      expect(await prefetch, isTrue);
      expect(fixture.catalog.strictReads, 1);
      expect(fixture.catalog.intents, hasLength(1));
      expect(fixture.state.windowStartItemOffset, 30);
    },
  );

  test(
    "a newer date owns publication and loading after an old completion",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final oldRead = Completer<LibraryTimeSnapshot>();
      final newRead = Completer<LibraryTimeSnapshot>();
      fixture.catalog.pendingResolution = oldRead;
      final old = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 10,
      );
      await _drain();
      final newer = fixture.viewport.jumpToTime(
        fixture.historical,
        itemOffset: 20,
      );
      fixture.catalog.pendingResolution = newRead;
      oldRead.complete(
        fixture.catalog.resolution(fixture.catalog.intents.first),
      );
      expect(await old, isFalse);
      await _drain();
      expect(fixture.state.isLoadingTimeAnchor, isTrue);
      expect(fixture.catalog.intents, hasLength(2));
      newRead.complete(
        fixture.catalog.resolution(fixture.catalog.intents.last),
      );
      expect(await newer, isTrue);
      expect(fixture.state.windowStartItemOffset, 40);
      expect(fixture.state.activeTimeAnchor!.itemOffset, 20);
      expect(fixture.state.isLoadingTimeAnchor, isFalse);
    },
  );

  test("a removed root cannot publish a date result", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.catalog.roots.removeWhere((root) => root.id == "other");
    expect(
      await fixture.viewport.jumpToTime(fixture.historical, itemOffset: 10),
      isFalse,
    );
    expect(fixture.state.catalogRevision, BigInt.one);
    expect(
      fixture.state.timeNavigationErrorMessage,
      contains("catalog_time_target_removed"),
    );
    expect(fixture.state.isLoadingTimeAnchor, isFalse);
  });
}

Future<void> _drain() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}

class _Fixture {
  _Fixture() {
    scanner.checkpoint = null;
    catalog = _PublishingCatalog(scanner);
    state = LibraryState.fromSnapshot(
      catalog.snapshot(query),
      query: query,
    ).copyWith(timeline: catalog.timeline(BigInt.one));
    viewport = LibraryViewportController(
      catalog,
      () => state,
      (next) => state = next,
      () => false,
      (_) {},
    );
    viewport.seed(state);
    catalog.revision = BigInt.two;
  }

  static const query = LibraryGalleryQuery(rootId: "other");
  final scanner = RetainedScanScanner();
  late final _PublishingCatalog catalog;
  late LibraryState state;
  late final LibraryViewportController viewport;
  LibraryTimeBucket get historical => state.timeline!.buckets.last;

  void dispose() {
    viewport.dispose();
    scanner.dispose();
  }
}

class _PublishingCatalog extends RetainedScanCatalog {
  _PublishingCatalog(super.scanner);

  static const stale = LibraryCatalogFailure(
    code: "catalog_cursor_stale",
    message: "Publication replaced the old date cursor",
  );
  BigInt revision = BigInt.one;
  int strictReads = 0;
  final intents = <LibraryTimeIntent>[];
  Completer<LibrarySnapshot>? pendingStrict;
  Completer<LibraryTimeSnapshot>? pendingResolution;

  @override
  LibrarySnapshot snapshot(LibraryGalleryQuery query, {int offset = 0}) {
    final original = super.snapshot(query, offset: offset);
    return LibrarySnapshot(
      catalogPath: original.catalogPath,
      revision: revision,
      queryId: original.queryId,
      roots: original.roots,
      assets: original.assets,
    );
  }

  LibraryTimeline timeline(BigInt version) => LibraryTimeline(
    revision: version,
    queryId: "query-other",
    totalItems: version == BigInt.one ? 100 : 50,
    buckets: [
      LibraryTimeBucket(
        monthKey: "2026-09",
        itemCount: version == BigInt.one ? 70 : 20,
        aspectRatioSum: version == BigInt.one ? 70 : 20,
      ),
      const LibraryTimeBucket(
        monthKey: "2012-03",
        itemCount: 30,
        aspectRatioSum: 30,
      ),
    ],
  );

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline(revision);

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) {
    strictReads++;
    return pendingStrict?.future ?? Future.error(stale);
  }

  LibraryTimeSnapshot resolution(LibraryTimeIntent intent) {
    final offset = 20 + intent.itemOffset;
    return LibraryTimeSnapshot(
      snapshot: snapshot(_Fixture.query, offset: offset),
      timeline: timeline(revision),
      windowStartItemOffset: offset,
      anchor: LibraryTimeAnchor(
        revision: revision,
        queryId: "query-other",
        monthKey: intent.monthKey,
        itemOffset: intent.itemOffset,
      ),
    );
  }

  @override
  Future<LibraryTimeSnapshot> resolveTimeIntent({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeIntent intent,
  }) {
    intents.add(intent);
    return pendingResolution?.future ?? Future.value(resolution(intent));
  }
}
