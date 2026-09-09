import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_query_snapshot_reader.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "query replacement consumes one coherent snapshot during catalog updates",
    () async {
      final catalog = _AtomicCatalog([Future.value(_querySnapshot(8))]);
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);

      expect(
        await controller.refreshFromSynchronization(
          catalogRevision: BigInt.from(8),
          anchorLocationId: "old-location",
          anchorAssetId: "asset",
          fallbackGlobalItemIndex: 19,
        ),
        LibraryQueryUpdateOutcome.applied,
      );
      expect(state.catalogRevision, BigInt.from(8));
      expect(state.timeline?.revision, BigInt.from(8));
      expect(catalog.requests, hasLength(1));
      expect(catalog.requests.single?.requestedLocationId, "old-location");
      expect(catalog.requests.single?.assetId, "asset");
      expect(catalog.requests.single?.fallbackGlobalItemIndex, 19);
      expect(catalog.separateReads, 0);
    },
  );

  test(
    "first page refresh does not assemble independent page and timeline reads",
    () async {
      final catalog = _AtomicCatalog([Future.value(_querySnapshot(9))]);
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);
      expect(await controller.reloadFirstCatalogPage(), isTrue);
      expect(state.catalogRevision, BigInt.from(9));
      expect(state.timeline?.revision, BigInt.from(9));
      expect(catalog.separateReads, 0);
    },
  );

  test(
    "a superseded coherent response cannot overwrite a newer query",
    () async {
      final oldResult = Completer<LibraryQuerySnapshot>();
      final catalog = _AtomicCatalog([
        oldResult.future,
        Future.value(_querySnapshot(9, queryId: "new")),
      ]);
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);
      final old = controller.updateQuery(
        const LibraryGalleryQuery(searchText: "old"),
      );
      expect(
        await controller.updateQuery(
          const LibraryGalleryQuery(searchText: "new"),
        ),
        isTrue,
      );
      oldResult.complete(_querySnapshot(8, queryId: "old"));
      expect(await old, isFalse);
      expect(state.query.searchText, "new");
      expect(state.catalogRevision, BigInt.from(9));
    },
  );

  test(
    "a changed deferred timeline waits for removal release before refreshing coherently",
    () async {
      final catalog = _RemovalCatalog();
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);
      final publication = controller.reserveCatalogPublication()!;
      expect(
        await controller.reloadCommittedRootRemovalFirstPage(
          removedRootId: "removed",
        ),
        isTrue,
      );
      expect(state.catalogRevision, BigInt.from(8));
      expect(state.isLoadingTimeline, isTrue);
      expect(catalog.pendingTimeline.isCompleted, isFalse);
      catalog.pendingTimeline.complete(_querySnapshot(9).timeline);
      await Future<void>.delayed(Duration.zero);
      expect(state.catalogRevision, BigInt.from(8));
      expect(catalog.requests, isEmpty);
      controller.releaseCatalogPublication(publication);
      await Future<void>.delayed(Duration.zero);
      expect(state.catalogRevision, BigInt.from(10));
      expect(state.timeline?.revision, BigInt.from(10));
      expect(state.timeNavigationErrorMessage, isNull);
      expect(catalog.requests, hasLength(1));
    },
  );

  test(
    "a replacement query retires the deferred removal refresh before another read",
    () async {
      final catalog = _RemovalCatalog();
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);
      final publication = controller.reserveCatalogPublication()!;
      expect(
        await controller.reloadCommittedRootRemovalFirstPage(
          removedRootId: "removed",
        ),
        isTrue,
      );
      catalog.pendingTimeline.complete(_querySnapshot(9).timeline);
      await Future<void>.delayed(Duration.zero);
      expect(catalog.requests, isEmpty);

      controller.releaseCatalogPublication(publication);
      expect(
        await controller.updateQuery(
          const LibraryGalleryQuery(searchText: "new query"),
        ),
        isTrue,
      );
      await Future<void>.delayed(Duration.zero);
      expect(state.query.searchText, "new query");
      expect(state.catalogRevision, BigInt.from(10));
      expect(state.timeline?.revision, state.catalogRevision);
      expect(state.isLoadingTimeline, isFalse);
      expect(state.timeNavigationErrorMessage, isNull);
      expect(catalog.requests, hasLength(1));
    },
  );

  test(
    "rejects inconsistent and too-old query evidence without retrying",
    () async {
      final inconsistent = LibraryQuerySnapshot(
        snapshot: _querySnapshot(7).snapshot,
        timeline: _querySnapshot(8).timeline,
      );
      final catalog = _AtomicCatalog([
        Future.value(inconsistent),
        Future.value(_querySnapshot(7)),
      ]);
      final reader = LibraryQuerySnapshotReader(catalog);
      await expectLater(
        reader.load(query: const LibraryGalleryQuery()),
        throwsA(
          isA<LibraryCatalogFailure>().having(
            (error) => error.code,
            "code",
            "catalog_query_snapshot_invalid",
          ),
        ),
      );
      await expectLater(
        reader.load(
          query: const LibraryGalleryQuery(),
          minimumRevision: BigInt.from(8),
        ),
        throwsA(
          isA<LibraryCatalogFailure>().having(
            (error) => error.code,
            "code",
            "catalog_revision_stale",
          ),
        ),
      );
      expect(catalog.requests, hasLength(2));
      expect(catalog.separateReads, 0);
    },
  );

  test(
    "a failed deferred refresh clears loading and retains the published removal page",
    () async {
      final result = Completer<LibraryQuerySnapshot>();
      final catalog = _RemovalCatalog(response: result.future);
      var state = LibraryState.fromSnapshot(_querySnapshot(7).snapshot);
      final controller = LibraryViewportController(
        catalog,
        () => state,
        (value) => state = value,
        () => false,
        (_) {},
      );
      addTearDown(controller.dispose);
      controller.seed(state);
      expect(
        await controller.reloadCommittedRootRemovalFirstPage(
          removedRootId: "removed",
        ),
        isTrue,
      );
      catalog.pendingTimeline.complete(_querySnapshot(9).timeline);
      await Future<void>.delayed(Duration.zero);
      expect(catalog.requests, hasLength(1));
      result.completeError(
        const LibraryCatalogFailure(
          code: "catalog_read_failed",
          message: "controlled failure",
        ),
      );
      await Future<void>.delayed(Duration.zero);
      expect(state.catalogRevision, BigInt.from(8));
      expect(state.roots.single.id, "kept");
      expect(state.timeline, isNull);
      expect(state.isLoadingTimeline, isFalse);
      expect(state.timeNavigationErrorMessage, contains("catalog_read_failed"));
      expect(catalog.requests, hasLength(1));
    },
  );
}

LibraryQuerySnapshot _querySnapshot(int revision, {String queryId = "query"}) =>
    LibraryQuerySnapshot(
      snapshot: LibrarySnapshot(
        catalogPath: "fixture.sqlite3",
        revision: BigInt.from(revision),
        queryId: queryId,
        roots: const [],
        assets: const [],
      ),
      timeline: LibraryTimeline(
        revision: BigInt.from(revision),
        queryId: queryId,
        totalItems: 0,
        buckets: const [],
      ),
    );

class _AtomicCatalog implements LibraryCatalog {
  _AtomicCatalog(this.responses);

  final List<Future<LibraryQuerySnapshot>> responses;
  final List<LibraryQueryAnchor?> requests = [];
  int separateReads = 0;

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) {
    requests.add(anchor);
    return responses.removeAt(0);
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) {
    separateReads += 1;
    throw StateError(
      "An independent page read can observe a newer publication",
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) {
    separateReads += 1;
    throw StateError(
      "An independent timeline read can observe a newer publication",
    );
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError();

  @override
  Future<bool> unregisterRoot(String rootId) => throw UnimplementedError();
}

class _RemovalCatalog extends _AtomicCatalog {
  _RemovalCatalog({Future<LibraryQuerySnapshot>? response})
    : super([response ?? Future.value(_querySnapshot(10))]);

  final Completer<LibraryTimeline> pendingTimeline = Completer();

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    return LibrarySnapshot(
      catalogPath: "fixture.sqlite3",
      revision: BigInt.from(8),
      queryId: "query",
      roots: const [
        LibraryRoot(
          id: "kept",
          path: "C:\\Fixture",
          displayPath: "C:\\Fixture",
          activeScanId: "scan",
          createdUnixMs: 0,
          assetCount: 0,
          issueCount: 0,
        ),
      ],
      assets: const [],
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) =>
      pendingTimeline.future;
}
