import "dart:async";

import "package:cedarflake_ame/features/library/application/library_workbench_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_browse_workspaces.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_review_fixture.dart";

void main() {
  late ReviewSurfaceFixture surface;
  late LibraryOrganizationMode review;
  late LibraryBrowseWorkspaces navigation;
  late LibraryWorkspace visible;

  setUp(() {
    surface = ReviewSurfaceFixture();
    review = LibraryOrganizationMode(ReviewServiceFixture(), surface);
    visible = LibraryWorkspace.gallery;
    navigation = LibraryBrowseWorkspaces(
      surface: surface,
      review: review,
      publish: (workspace) => visible = workspace,
    );
  });
  tearDown(() {
    navigation.dispose();
    review.dispose();
  });

  Future<void> visitAlbum() async {
    await navigation.select(LibraryWorkspace.albums);
    surface.library = surface.library.copyWith(
      query: surface.library.query.copyWith(
        collection: const LibraryGalleryCollection.album("a"),
        rootId: null,
      ),
    );
  }

  test(
    "ordinary destination switches retain scope and reset position and selection",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(rootId: "source"),
      );
      await navigation.select(LibraryWorkspace.gallery);
      expect(surface.restored, isNull);
      await visitAlbum();
      expect(surface.restored!.isSelecting, isFalse);
      expect(surface.restored!.selection.isEmpty, isTrue);
      expect(surface.queries, isEmpty);
      await navigation.select(LibraryWorkspace.gallery);
      expect(surface.library.query.rootId, "source");
      expect(surface.library.query.collection, isA<LibraryAllCollection>());
      expect(surface.restored!.isSelecting, isFalse);
      expect(surface.restored!.selection.isEmpty, isTrue);
      expect(surface.positions.last, isNull);
      await navigation.select(LibraryWorkspace.albums);
      expect(surface.restored!.isSelecting, isFalse);
      expect(surface.restored!.selection.isEmpty, isTrue);
      expect(surface.positions.last, isNull);
      expect(
        surface.library.query.collection,
        const LibraryGalleryCollection.album("a"),
      );
      expect(surface.library.query.rootId, isNull);
      final reads = surface.queries.length;
      await navigation.select(LibraryWorkspace.settings);
      await navigation.select(LibraryWorkspace.albums);
      expect(visible, LibraryWorkspace.albums);
      expect(surface.queries.length, reads + 1);
    },
  );

  test(
    "returning to Library restores organization mode scope position and selection",
    () async {
      await review.enter();
      expect(review.isActive, isTrue);
      await review.changeQuery(
        surface.library.query.copyWith(rootId: "review-source"),
      );
      final organized = surface.captureBrowse();
      await visitAlbum();
      expect(review.isActive, isFalse);
      await navigation.select(LibraryWorkspace.gallery);
      expect(
        surface.library.query.organizationQueue,
        LibraryOrganizationQueue.pending,
      );
      expect(surface.library.query.rootId, "review-source");
      expect(review.isActive, isTrue);
      expect(surface.restored!.isSelecting, isTrue);
      expect(
        surface.restored!.selection.includedAssetIds,
        organized.selection.includedAssetIds,
      );
      expect(surface.positions.last!.globalItemIndex, 23);
      expect(surface.positions.last!.itemFraction, .3);
      await review.exit();
      await visitAlbum();
      await navigation.select(LibraryWorkspace.gallery);
      expect(surface.restored!.isSelecting, isFalse);
      expect(surface.restored!.selection.isEmpty, isTrue);
      expect(surface.positions.last, isNull);
    },
  );

  for (final destination in [
    LibraryWorkspace.duplicates,
    LibraryWorkspace.settings,
  ]) {
    test(
      "ordinary Library resets after visiting ${destination.name}",
      () async {
        surface.library = surface.library.copyWith(
          query: surface.library.query.copyWith(rootId: "source"),
        );
        await navigation.select(destination);
        expect(visible, destination);
        expect(surface.queries, isEmpty);
        await navigation.select(LibraryWorkspace.gallery);
        expect(visible, LibraryWorkspace.gallery);
        expect(surface.library.query.rootId, "source");
        expect(surface.restored!.selection.isEmpty, isTrue);
        expect(surface.restored!.isSelecting, isFalse);
        expect(surface.positions.single, isNull);
      },
    );

    test("organizing Library survives a ${destination.name} visit", () async {
      await review.enter();
      final session = review.session;
      final reads = surface.queries.length;
      await navigation.select(destination);
      expect(visible, destination);
      await navigation.select(LibraryWorkspace.gallery);
      expect(visible, LibraryWorkspace.gallery);
      expect(review.isActive, isTrue);
      expect(identical(review.session, session), isTrue);
      expect(surface.queries.length, reads);
      expect(surface.restored, isNull);
    });
  }

  test(
    "failed organization restoration retains its snapshot for retry",
    () async {
      await review.enter();
      await visitAlbum();
      surface.publishing = () async => false;
      await navigation.select(LibraryWorkspace.gallery);
      expect(visible, LibraryWorkspace.albums);
      expect(review.isActive, isFalse);
      surface.publishing = null;
      expect(await navigation.retry(surface.queries.last), isTrue);
      expect(visible, LibraryWorkspace.gallery);
      expect(review.isActive, isTrue);
      expect(surface.restored!.isSelecting, isTrue);
      expect(surface.positions.last!.globalItemIndex, 23);
    },
  );

  test(
    "a failed restoration keeps the current destination and allows retry",
    () async {
      await visitAlbum();
      surface.publishing = () async => false;
      await navigation.select(LibraryWorkspace.gallery);
      expect(visible, LibraryWorkspace.albums);
      surface.publishing = null;
      expect(await navigation.retry(surface.queries.last), isTrue);
      expect(visible, LibraryWorkspace.gallery);
    },
  );

  test(
    "the latest destination wins while a previous restoration finishes",
    () async {
      await visitAlbum();
      final pending = Completer<bool>();
      surface.publishing = () => pending.future;
      final switching = navigation.select(LibraryWorkspace.gallery);
      final returning = navigation.select(LibraryWorkspace.albums);
      pending.complete(true);
      await switching;
      await returning;
      expect(visible, LibraryWorkspace.albums);
      expect(
        surface.library.query.collection,
        const LibraryGalleryCollection.album("a"),
      );
    },
  );

  test(
    "retiring an album invalidates saved and in-flight restoration",
    () async {
      await visitAlbum();
      await navigation.select(LibraryWorkspace.gallery);
      final pending = Completer<bool>();
      surface.publishing = () => pending.future;
      final switching = navigation.select(LibraryWorkspace.albums);
      await navigation.retireAlbum("a");
      pending.complete(true);
      await switching;
      expect(visible, LibraryWorkspace.gallery);
      expect(surface.library.query.collection, isA<LibraryAllCollection>());
      await navigation.select(LibraryWorkspace.albums);
      expect(surface.library.query.collection, isA<LibraryAllCollection>());
    },
  );

  test("disposal retires pending destination publication", () async {
    await visitAlbum();
    final pending = Completer<bool>();
    surface.publishing = () => pending.future;
    final switching = navigation.select(LibraryWorkspace.gallery);
    navigation.dispose();
    pending.complete(true);
    await switching;
    expect(visible, LibraryWorkspace.albums);
  });
}
