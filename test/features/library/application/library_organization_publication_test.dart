import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_publication.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationLibraryFixture library;
  late ProviderContainer container;
  late LibraryOrganizationPublication publication;
  late _Projection projection;
  var disposed = false;

  setUp(() {
    disposed = false;
    library = OrganizationLibraryFixture();
    container = ProviderContainer(
      overrides: [libraryControllerProvider.overrideWith(() => library)],
    );
    publication = container.read(libraryOrganizationPublicationProvider);
    projection = _Projection();
    publication.attach(projection);
  });
  tearDown(() {
    if (!disposed) container.dispose();
  });

  bool hasRefreshFailure() {
    final notifications = container.read(ameNotificationControllerProvider);
    return notifications.history.any(
      (entry) =>
          entry.actionId == "organization.refresh" &&
          notifications.pendingIds.contains(entry.id),
    );
  }

  test(
    "a committed retirement leaves its scope before refreshing the catalog",
    () async {
      final retired = Completer<bool>();
      projection.retiring = (_) => retired.future;
      final refreshing = publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      await Future<void>.delayed(Duration.zero);
      expect(projection.albums, ["selected"]);
      expect(library.refreshes, 0);
      retired.complete(true);
      await refreshing;
      expect(library.refreshes, 1);
      expect(hasRefreshFailure(), isFalse);
    },
  );

  test(
    "failed scope retirement retries without reading a deleted album",
    () async {
      projection.retiring = (_) async => false;
      await publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      expect(library.refreshes, 0);
      expect(hasRefreshFailure(), isTrue);
      projection.retiring = null;
      await publication.refresh();
      expect(projection.albums, ["selected", "selected"]);
      expect(library.refreshes, 1);
      expect(hasRefreshFailure(), isFalse);
      await publication.refresh();
      expect(projection.albums, hasLength(2));
      expect(library.refreshes, 2);
    },
  );

  test(
    "a failed read retains the committed retirement for its retry",
    () async {
      library.refreshSucceeds = false;
      await publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      expect(hasRefreshFailure(), isTrue);
      library.refreshSucceeds = true;
      await publication.refresh();
      expect(projection.albums, ["selected", "selected"]);
      expect(hasRefreshFailure(), isFalse);
    },
  );

  test(
    "detaching an old gallery preserves the replacement projection",
    () async {
      final previous = publication.attach(projection);
      final replacement = _Projection();
      publication.attach(replacement);
      publication.detach(previous);
      await publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      expect(projection.albums, isEmpty);
      expect(replacement.albums, ["selected"]);
      expect(library.refreshes, 1);
    },
  );

  test(
    "late projection completion cannot satisfy its replacement's obligation",
    () async {
      final retired = Completer<bool>();
      projection.retiring = (_) => retired.future;
      final refreshing = publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      await Future<void>.delayed(Duration.zero);
      final replacement = _Projection();
      publication.attach(replacement);
      retired.complete(true);
      await refreshing;
      expect(library.refreshes, 0);
      expect(hasRefreshFailure(), isTrue);
      await publication.refresh();
      expect(replacement.albums, ["selected"]);
      expect(library.refreshes, 1);
      expect(hasRefreshFailure(), isFalse);
    },
  );

  test(
    "overlapping commits serialize scope retirement before their reads",
    () async {
      final retired = Completer<bool>();
      projection.retiring = (_) => retired.future;
      final first = publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      await Future<void>.delayed(Duration.zero);
      final second = publication.refresh();
      await Future<void>.delayed(Duration.zero);
      expect(projection.albums, ["selected"]);
      expect(library.refreshes, 0);
      expect(hasRefreshFailure(), isFalse);
      retired.complete(true);
      await Future.wait([first, second]);
      expect(projection.albums, ["selected"]);
      expect(library.refreshes, 2);
      expect(hasRefreshFailure(), isFalse);
    },
  );

  test(
    "disposal retires pending publication without a catalog read or notice",
    () async {
      final retired = Completer<bool>();
      projection.retiring = (_) => retired.future;
      final refreshing = publication.refresh(
        const LibraryOrganizationChange.albumRetired("selected"),
      );
      await Future<void>.delayed(Duration.zero);
      container.dispose();
      disposed = true;
      retired.complete(true);
      await refreshing;
      expect(library.refreshes, 0);
    },
  );
}

class _Projection implements LibraryOrganizationProjection {
  final List<String> albums = [];
  Future<bool> Function(String)? retiring;

  @override
  Future<bool> retireAlbum(String albumId) async {
    albums.add(albumId);
    return retiring?.call(albumId) ?? true;
  }
}
