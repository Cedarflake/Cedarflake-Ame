import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_album_requests.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/application/library_organization_outcome.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  late LibraryOrganizationActions actions;
  setUp(() {
    service = OrganizationServiceFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
    actions = container.read(libraryOrganizationActionsProvider.notifier);
  });
  tearDown(() => container.dispose());

  test(
    "unknown album receipt retries the same operation and returns form feedback",
    () async {
      final edit = LibraryAlbumEditSession(
        kind: LibraryAlbumEditKind.create,
        revision: BigInt.one,
      );
      service.saveAlbum = (_) async => throw StateError("lost receipt");
      final failed = await actions.submit(edit.request("  Landscape  "));
      expect(failed, isA<LibraryOrganizationRejected>());
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(container.read(libraryOrganizationOutcomeProvider), isNull);
      service.saveAlbum = null;
      final saved = await actions.submit(edit.request("Landscape"));
      expect(saved, isA<LibraryOrganizationSaved>());
      expect(service.albumWrites[1].id, service.albumWrites[0].id);
      expect(service.albumWrites[1].revision, BigInt.one);
      expect(service.albumItems.single.name, "Landscape");
      expect(
        container.read(libraryOrganizationOutcomeProvider)?.actionId,
        "organization.undo",
      );
    },
  );

  test("stale rename refreshes authority only on explicit retry", () async {
    final edit = LibraryAlbumEditSession(
      kind: LibraryAlbumEditKind.rename,
      revision: BigInt.one,
      album: const LibraryOrganizationAlbum("a", "Old"),
    );
    service.saveAlbum = (_) async => throw const LibraryOrganizationFailure(
      "organization_selection_changed",
      "stale",
    );
    await actions.submit(edit.request("New"));
    expect(service.albumWrites, hasLength(1));
    service.saveAlbum = null;
    await actions.submit(edit.request("New"));
    expect(service.albumWrites[1].id, isNot(service.albumWrites[0].id));
    expect(service.albumWrites[1].revision, BigInt.two);
    expect(service.albumWrites[1].albumId, "a");
  });

  test(
    "empty album deletion retains undo without manufacturing changed groups",
    () async {
      const album = LibraryOrganizationAlbum("a", "Empty");
      service.albumItems = [album];
      final edit = LibraryAlbumEditSession(
        kind: LibraryAlbumEditKind.delete,
        revision: BigInt.one,
        album: album,
      );
      final result =
          await actions.submit(edit.request("Empty"))
              as LibraryOrganizationSaved;
      expect(result.receipt.changedGroups, 0);
      expect(service.albumItems, isEmpty);
      final undo = container.read(libraryOrganizationOutcomeProvider)!;
      actions.handleNotification(undo);
      await Future<void>.delayed(Duration.zero);
      expect(service.undos.single.originalId, result.receipt.operationId);
    },
  );

  test(
    "membership retry retains prepared groups and leaves favorites unchanged",
    () async {
      service.isFavorite = true;
      final session = LibraryAlbumMembershipSession([
        organizationTarget("one"),
      ], isMove: true);
      const album = LibraryOrganizationAlbum("destination", "New");
      service.saveMembership = (_) async => throw StateError("unknown receipt");
      await actions.submit(session.request(album));
      service.saveMembership = null;
      await actions.submit(session.request(album));
      expect(service.preparationCount, 1);
      expect(service.memberships[1].id, service.memberships[0].id);
      expect(
        service.memberships[1].prepared,
        same(service.memberships[0].prepared),
      );
      expect(service.memberships[1].albumId, "destination");
      expect(service.isFavorite, isTrue);
      await actions.submit(session.request(null));
      expect(service.memberships.last.albumId, isNull);
      expect(service.memberships.last.id, isNot(service.memberships.first.id));
    },
  );

  test(
    "closing a picker during preparation prevents its later write",
    () async {
      service.preparing = Completer<LibraryOrganizationPrepared>();
      final session = LibraryAlbumMembershipSession([
        organizationTarget("one"),
      ], isMove: false);
      final saving = actions.submit(
        session.request(const LibraryOrganizationAlbum("a", "Album")),
      );
      await Future<void>.delayed(Duration.zero);
      session.dispose();
      service.preparing!.complete(const OrganizationPreparedFixture());
      expect(await saving, isA<LibraryOrganizationRetired>());
      expect(service.memberships, isEmpty);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(
        container.read(ameNotificationControllerProvider).hasUnread,
        isFalse,
      );
    },
  );
}
