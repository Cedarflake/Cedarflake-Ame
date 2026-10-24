import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_query_transition.dart";
import "package:cedarflake_ame/features/library/presentation/library_review_query_transition.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_position.dart";
import "package:flutter_test/flutter_test.dart";

import "support/library_screen_fixture.dart";

void main() {
  for (final emptyLibrary in [true, false]) {
    test(
      "returning to a retained first window avoids a read, empty=$emptyLibrary",
      () async {
        final fixture = _Fixture(
          emptyLibrary
              ? const LibraryState()
              : populatedScreenState(totalItems: 12),
        );
        addTearDown(fixture.dispose);
        expect(await fixture.review.show(fixture.state.query), isTrue);
        expect(fixture.catalog.loadQueries, isEmpty);
        expect(fixture.publications.single.preservePosition, isFalse);
        expect(fixture.state.windowStartItemOffset, 0);
      },
    );
  }

  test("returning from a later window reloads the beginning", () async {
    final fixture = _Fixture(
      populatedScreenState(
        totalItems: 300,
      ).copyWith(windowStartItemOffset: 256),
    );
    addTearDown(fixture.dispose);
    expect(await fixture.review.show(fixture.state.query), isTrue);
    expect(fixture.catalog.loadQueries, hasLength(1));
    expect(fixture.state.windowStartItemOffset, 0);
    expect(fixture.publications.single.preservePosition, isFalse);
  });

  test("changing collection still loads its own first window", () async {
    final fixture = _Fixture(populatedScreenState(totalItems: 12));
    addTearDown(fixture.dispose);
    final query = fixture.state.query.copyWith(
      collection: const LibraryGalleryCollection.favorites(),
    );
    expect(await fixture.review.show(query), isTrue);
    expect(fixture.catalog.loadQueries, [query]);
    expect(fixture.state.query, query);
    expect(fixture.publications.single.preservePosition, isFalse);
  });

  for (final (name, state) in [
    ("following page", const LibraryState(isLoadingPage: true)),
    ("preceding page", const LibraryState(isLoadingPreviousPage: true)),
    ("visible range", const LibraryState(isLoadingVisibleRange: true)),
    (
      "failed query",
      const LibraryState(
        queryActivity: LibraryQueryFailed(
          requestedQuery: LibraryGalleryQuery(),
          message: "Interrupted read",
        ),
      ),
    ),
  ]) {
    test("$name takes the refresh path", () async {
      final fixture = _Fixture(state);
      addTearDown(fixture.dispose);
      expect(await fixture.review.show(state.query), isTrue);
      expect(fixture.catalog.loadQueries, hasLength(1));
      expect(fixture.state.queryActivity, isA<LibraryQueryIdle>());
      expect(fixture.publications.single.preservePosition, isFalse);
    });
  }

  test("an active explicit time seek retains its admission gate", () async {
    final fixture = _Fixture(const LibraryState(isLoadingTimeAnchor: true));
    addTearDown(fixture.dispose);
    expect(await fixture.review.show(fixture.state.query), isFalse);
    expect(fixture.catalog.loadQueries, isEmpty);
    expect(fixture.publications, isEmpty);
  });

  test("a completed time seek reloads and clears its anchor", () async {
    final fixture = _Fixture(
      populatedScreenState(totalItems: 12).copyWith(
        activeTimeAnchor: LibraryTimeAnchor(
          revision: BigInt.one,
          queryId: "query-1",
          itemOffset: 0,
        ),
      ),
    );
    addTearDown(fixture.dispose);
    expect(await fixture.review.show(fixture.state.query), isTrue);
    expect(fixture.catalog.loadQueries, hasLength(1));
    expect(fixture.state.activeTimeAnchor, isNull);
  });

  test(
    "a restored position and an explicit review target require fresh reads",
    () async {
      final fixture = _Fixture(populatedScreenState(totalItems: 12));
      addTearDown(fixture.dispose);
      final asset = fixture.state.assets.first;
      final position = LibraryGalleryVisiblePosition(
        queryId: fixture.state.queryId,
        revision: fixture.state.catalogRevision!,
        monthKey: null,
        locationId: asset.locationId,
        assetId: asset.assetId,
        globalItemIndex: 0,
        itemFraction: .3,
        viewportFraction: .4,
      );
      expect(
        await fixture.review.show(fixture.state.query, position: position),
        isTrue,
      );
      expect(fixture.catalog.loadQueries, hasLength(1));
      expect(fixture.publications.single.preservePosition, isTrue);
      expect(fixture.publications.single.position!.itemFraction, .3);
      expect(
        await fixture.review.show(
          fixture.state.query,
          target: LibraryOrganizationTarget.fromAsset(asset),
        ),
        isTrue,
      );
      expect(fixture.catalog.loadQueries, hasLength(2));
      expect(fixture.publications.last.preservePosition, isTrue);
    },
  );
}

class _Fixture {
  _Fixture(this.state) {
    catalog = RecordingQueryCatalog(
      LibrarySnapshot(
        catalogPath: "controlled.sqlite3",
        revision: state.catalogRevision ?? BigInt.one,
        queryId: state.queryId,
        roots: state.roots,
        assets: state.assets,
      ),
    );
    viewport = LibraryViewportController(
      catalog,
      () => state,
      (next) => state = next,
      () => false,
      (_) {},
    );
    position = LibraryGalleryQueryTransition(
      readState: () => state,
      capturePosition: (_) => null,
      readViewerAnchor: () => null,
      reconcileViewer: () async {},
      onPublished: publications.add,
      onFailed: () {},
    );
    review = LibraryReviewQueryTransition(
      readState: () => state,
      updateQuery: viewport.updateQuery,
      positionTransition: position,
    );
  }

  LibraryState state;
  late final RecordingQueryCatalog catalog;
  late final LibraryViewportController viewport;
  late final LibraryGalleryQueryTransition position;
  late final LibraryReviewQueryTransition review;
  final publications = <LibraryGalleryQueryPublication>[];

  void dispose() {
    position.dispose();
    viewport.dispose();
  }
}
