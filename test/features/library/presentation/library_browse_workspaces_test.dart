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
    "destinations retain independent scope, selection and position",
    () async {
      surface.library = surface.library.copyWith(
        query: surface.library.query.copyWith(rootId: "source"),
      );
      await visitAlbum();
      expect(surface.queries, isEmpty);
      await navigation.select(LibraryWorkspace.gallery);
      expect(surface.library.query.rootId, "source");
      expect(surface.library.query.collection, isA<LibraryAllCollection>());
      expect(surface.restored!.isSelecting, isTrue);
      expect(surface.positions.last!.globalItemIndex, 23);
      expect(surface.positions.last!.itemFraction, .3);
      await navigation.select(LibraryWorkspace.albums);
      expect(
        surface.library.query.collection,
        const LibraryGalleryCollection.album("a"),
      );
      expect(surface.library.query.rootId, isNull);
      final reads = surface.queries.length;
      await navigation.select(LibraryWorkspace.settings);
      await navigation.select(LibraryWorkspace.albums);
      expect(visible, LibraryWorkspace.albums);
      expect(surface.queries.length, reads);
    },
  );

  test(
    "switching browsing destinations exits review and restores ordinary scope",
    () async {
      await review.enter();
      expect(review.isActive, isTrue);
      await visitAlbum();
      expect(review.isActive, isFalse);
      await navigation.select(LibraryWorkspace.gallery);
      expect(surface.library.query.organizationQueue, isNull);
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
