import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_review_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  late ReviewServiceFixture service;
  late ReviewSurfaceFixture surface;
  late LibraryOrganizationMode mode;
  setUp(() {
    service = ReviewServiceFixture();
    surface = ReviewSurfaceFixture();
    mode = LibraryOrganizationMode(service, surface);
  });
  tearDown(() => mode.dispose());

  test(
    "review inherits sorting and preserves its visible anchor when reordered",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(
          sortKey: LibraryGallerySortKey.fileName,
          sortDirection: LibraryGallerySortDirection.ascending,
        ),
      );
      final ordinary = surface.library.query;
      await mode.enter();
      expect(surface.library.query.sortKey, ordinary.sortKey);
      expect(surface.library.query.sortDirection, ordinary.sortDirection);
      final position = surface.captureBrowse().position!;
      final sorted = surface.library.query.copyWith(
        sortKey: LibraryGallerySortKey.modifiedTime,
      );
      expect(await mode.changeQuery(sorted), isTrue);
      expect(surface.positions.last!.locationId, position.locationId);
      expect(
        surface.positions.last!.viewportFraction,
        position.viewportFraction,
      );
      expect(service.continuationQueries.last, sorted);
      expect(mode.ordinaryQuery, ordinary);
      await mode.selectQueue(LibraryOrganizationQueue.deferred);
      expect(surface.positions.last, isNull);
      expect(surface.library.query.sortKey, sorted.sortKey);
    },
  );

  test(
    "retiring the viewed album selects the library and preserves sorting",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(
          collection: const LibraryGalleryCollection.album("retired"),
          rootId: "root",
          searchText: "filter",
          sortKey: LibraryGallerySortKey.fileName,
        ),
      );
      expect(await mode.retireAlbum("retired"), isTrue);
      expect(
        surface.library.query.collection,
        const LibraryGalleryCollection.library(),
      );
      expect(surface.library.query.rootId, isNull);
      expect(surface.library.query.searchText, "filter");
      expect(surface.library.query.sortKey, LibraryGallerySortKey.fileName);
      expect(mode.isActive, isFalse);
    },
  );

  for (final collection in const [
    LibraryGalleryCollection.album("album"),
    LibraryGalleryCollection.favorites(),
  ]) {
    test("collections cannot enter organization mode: $collection", () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(collection: collection),
      );
      expect(mode.canEnter, isFalse);
      await mode.enter();
      expect(mode.isActive, isFalse);
      expect(surface.queries, isEmpty);
      expect(service.starts, isEmpty);
    });

    test(
      "active organization cannot enter a collection: $collection",
      () async {
        await mode.enter();
        final current = mode.session;
        expect(
          await mode.changeQuery(
            surface.library.query.copyWith(collection: collection),
          ),
          isFalse,
        );
        expect(surface.library.query.collection, isA<LibraryAllCollection>());
        expect(mode.session, same(current));
      },
    );
  }

  test("library and folder queues keep their selected source scope", () async {
    await mode.enter();
    expect(surface.library.query.rootId, isNull);
    expect(surface.library.query.folderRelativePath, isNull);
    final folder = surface.library.query.copyWith(
      rootId: "root-1",
      folderRelativePath: "photos/landscapes",
      includeDescendants: true,
    );
    expect(await mode.changeQuery(folder), isTrue);
    await mode.selectQueue(LibraryOrganizationQueue.deferred);
    expect(surface.library.query.rootId, "root-1");
    expect(surface.library.query.folderRelativePath, "photos/landscapes");
    expect(surface.library.query.includeDescendants, isTrue);
    expect(mode.session!.query, surface.library.query);
    await mode.changeQuery(
      surface.library.query.copyWith(rootId: null, folderRelativePath: null),
    );
    expect(surface.library.query.rootId, isNull);
    expect(surface.library.query.folderRelativePath, isNull);
  });

  for (final (index, foreign) in const [
    LibraryGalleryQuery(rootId: "other-root"),
    LibraryGalleryQuery(rootId: "root-1", folderRelativePath: "other-folder"),
    LibraryGalleryQuery(),
    LibraryGalleryQuery(collection: LibraryGalleryCollection.favorites()),
    LibraryGalleryQuery(collection: LibraryGalleryCollection.album("old")),
  ].indexed) {
    test(
      "continuation rejects a different source scope (case $index)",
      () async {
        final original = surface.library.query.copyWith(
          rootId: "root-1",
          folderRelativePath: "photos",
        );
        surface.library = surface.library.copyWith(query: original);
        service.saved = LibraryReviewContinuation(
          LibraryContinuationStatus.ready,
          query: foreign.copyWith(
            organizationQueue: LibraryOrganizationQueue.pending,
          ),
          target: LibraryOrganizationTarget.fromAsset(
            surface.library.assets.single,
          ),
        );
        await mode.enter();
        expect(mode.continuation?.canResume, isFalse);
        await mode.resume();
        expect(surface.opened, isEmpty);
        expect(surface.library.query.rootId, original.rootId);
        expect(
          surface.library.query.folderRelativePath,
          original.folderRelativePath,
        );
      },
    );
  }

  test(
    "library continuation cannot narrow all folders to a saved source",
    () async {
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query.copyWith(
          rootId: "root-1",
          organizationQueue: LibraryOrganizationQueue.pending,
        ),
        target: LibraryOrganizationTarget.fromAsset(
          surface.library.assets.single,
        ),
      );
      await mode.enter();
      expect(mode.continuation?.canResume, isFalse);
      await mode.resume();
      expect(surface.library.query.rootId, isNull);
      expect(surface.opened, isEmpty);
    },
  );

  test("failed retirement keeps the active scope retryable", () async {
    surface.library = surface.library.copyWith(
      query: surface.library.query.copyWith(
        collection: const LibraryGalleryCollection.album("retired"),
      ),
    );
    surface.publishing = () async => false;
    expect(await mode.retireAlbum("retired"), isFalse);
    expect(
      surface.library.query.collection,
      const LibraryGalleryCollection.album("retired"),
    );
    surface.publishing = null;
    expect(await mode.retireAlbum("retired"), isTrue);
    expect(
      surface.library.query.collection,
      const LibraryGalleryCollection.library(),
    );
  });

  test(
    "conflict notification opens the global deferred queue and preserves ordinary context",
    () async {
      final original = surface.library.query.copyWith(
        rootId: "one-root",
        searchText: "filter",
      );
      surface.library = surface.library.copyWith(query: original);
      await mode.showConflicts();
      expect(mode.isActive, isTrue);
      expect(mode.queue, LibraryOrganizationQueue.deferred);
      expect(surface.library.query.rootId, isNull);
      expect(surface.library.query.searchText, isEmpty);
      expect(surface.closes, 1);
      expect(service.starts, isEmpty);
      await mode.exit();
      expect(surface.library.query, original);
    },
  );

  test(
    "newer navigation supersedes a pending retired-album transition",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(
          collection: const LibraryGalleryCollection.album("retired"),
        ),
      );
      final moved = Completer<bool>();
      surface.publishing = () => moved.future;
      final retirement = mode.retireAlbum("retired");
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(
          collection: const LibraryGalleryCollection.favorites(),
        ),
      );
      moved.complete(false);
      expect(await retirement, isTrue);
      expect(
        surface.library.query.collection,
        const LibraryGalleryCollection.favorites(),
      );
    },
  );

  test(
    "viewer-admitted representative replacement renews authority without duplicate checkpoints",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final previous = mode.current;
      final replacement = galleryAsset(
        id: "replacement",
        captureLocalTime: "2020-01-01T00:00:00",
      );
      mode.reconcileDisplayedAsset(replacement);
      mode.adoptDisplayedAsset(replacement);
      await Future<void>.delayed(Duration.zero);
      expect(mode.current, isNot(same(previous)));
      expect(service.checkpoints, hasLength(2));
      expect(service.checkpoints.last.target.locationId, "replacement");
      expect(mode.session!.owns(previous!), isFalse);
    },
  );

  test(
    "catalog publications coalesce continuation reads and retire old results",
    () async {
      final old = Completer<LibraryReviewContinuation>();
      var reads = 0;
      service.saved = const LibraryReviewContinuation(
        LibraryContinuationStatus.emptyScope,
      );
      service.reading = () async => ++reads == 1 ? old.future : service.saved;
      final entering = mode.enter();
      await Future<void>.delayed(Duration.zero);
      final updates = List.generate(20, (_) => mode.refreshContinuation());
      expect(reads, 1);
      old.complete(
        const LibraryReviewContinuation(LibraryContinuationStatus.noActivity),
      );
      await entering;
      await Future.wait(updates);
      expect(reads, 2);
      expect(mode.continuation?.status, LibraryContinuationStatus.emptyScope);
    },
  );

  test(
    "entry and queue switching create no activity and exit restores ordinary context",
    () async {
      final query = surface.library.query.copyWith(
        sortKey: LibraryGallerySortKey.fileName,
        sortDirection: LibraryGallerySortDirection.ascending,
      );
      surface.library = surface.library.copyWith(query: query);
      await mode.enter();
      expect(mode.queue, LibraryOrganizationQueue.pending);
      expect(mode.continuation?.canResume, isFalse);
      await mode.selectQueue(LibraryOrganizationQueue.ignored);
      expect(mode.queue, LibraryOrganizationQueue.ignored);
      expect(service.starts, isEmpty);
      await mode.exit();
      expect(mode.isActive, isFalse);
      expect(surface.library.query, query);
      expect(surface.restored!.selection.includedAssetIds, {
        surface.library.assets.first.assetId,
      });
      expect(surface.positions.last!.globalItemIndex, 23);
      expect(surface.positions.last!.itemFraction, .3);
      expect(surface.positions.last!.viewportFraction, .4);
    },
  );

  test(
    "resume restores the saved queue and identity within the current folder",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(
          rootId: "root-1",
          folderRelativePath: "photos",
        ),
      );
      final target = LibraryOrganizationTarget.fromAsset(
        surface.library.assets.single,
      );
      final savedScope = surface.library.query.copyWith(
        organizationQueue: LibraryOrganizationQueue.deferred,
        searchText: "saved",
      );
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: savedScope,
        target: target,
      );
      await mode.enter();
      await mode.resume();
      await Future<void>.delayed(Duration.zero);
      expect(surface.targets.last, target);
      expect(surface.library.query, savedScope);
      expect(surface.opened.single.assetId, target.assetId);
      expect(service.checkpoints.single.target, target);
    },
  );

  test(
    "new arrivals update continuation availability without moving the open image",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final entry = mode.current;
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: LibraryOrganizationTarget.fromAsset(
          surface.library.assets.single,
        ),
      );
      await mode.refreshContinuation();
      expect(mode.current, same(entry));
      expect(surface.opened, isEmpty);
      expect(service.checkpoints, hasLength(1));
    },
  );

  test("save completion advances only its current committed entry", () async {
    await mode.enter();
    mode.open(surface.library.assets.single);
    await Future<void>.delayed(Duration.zero);
    final entry = mode.current!;
    await mode.afterSave(entry);
    expect(surface.closes, 0);
    mode.session!.accept(
      entry,
      LibraryReviewReceipt(
        LibraryOrganizationReceipt(
          operationId: "saved",
          revision: BigInt.two,
          changedGroups: 1,
        ),
        service.checkpoints.last.position.next(),
      ),
    );
    service.saved = const LibraryReviewContinuation(
      LibraryContinuationStatus.emptyScope,
    );
    await mode.afterSave(entry);
    expect(surface.closes, 1);
    expect(mode.current, isNull);
  });

  test(
    "closing the viewer during continuation loading cannot reopen it",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final entry = mode.current!;
      mode.session!.accept(
        entry,
        LibraryReviewReceipt(
          LibraryOrganizationReceipt(
            operationId: "saved",
            revision: BigInt.two,
            changedGroups: 1,
          ),
          service.checkpoints.last.position.next(),
        ),
      );
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: entry.target,
      );
      final publish = Completer<bool>();
      surface.publishing = () => publish.future;
      final advance = mode.afterSave(entry);
      await Future<void>.delayed(Duration.zero);
      mode.closeCurrent();
      publish.complete(true);
      await advance;
      expect(surface.opened, isEmpty);
    },
  );

  test(
    "a failed resume can retry the same workflow and keeps ordinary context",
    () async {
      await mode.enter();
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: LibraryOrganizationTarget.fromAsset(
          surface.library.assets.single,
        ),
      );
      surface.publishing = () async => false;
      await mode.resume();
      expect(mode.problem, isNotNull);
      expect(surface.opened, isEmpty);
      surface.publishing = null;
      await mode.retry();
      expect(surface.opened, hasLength(1));
      expect(mode.problem, isNull);
    },
  );
}
