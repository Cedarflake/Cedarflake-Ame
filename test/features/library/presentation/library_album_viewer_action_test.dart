import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_viewer_action.dart";
import "package:cedarflake_ame/features/settings/application/album_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  const current = LibraryOrganizationAlbum("current", "Current");
  const destination = LibraryOrganizationAlbum("destination", "Destination");

  setUp(() {
    service = OrganizationServiceFixture()..albumItems = [current, destination];
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
  });
  tearDown(() => container.dispose());

  Future<void> host(WidgetTester tester, String target) => tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        home: Scaffold(
          body: LibraryAlbumViewerAction(target: organizationTarget(target)),
        ),
      ),
    ),
  );

  Future<void> setPreferences(AlbumPreferences preferences) => container
      .read(amePreferencesControllerProvider.notifier)
      .update(AmePreferences(albums: preferences));

  testWidgets(
    "viewer move reflects the current album and waits for confirmation",
    (tester) async {
      service.inspectTarget = (_) async => LibraryOrganizationInspection(
        revision: BigInt.one,
        decisions: const [
          LibraryOrganizationDecision(
            favorite: true,
            disposition: LibraryOrganizationDisposition.organized,
            album: current,
          ),
        ],
      );
      await setPreferences(
        const AlbumPreferences(defaultAlbumId: "destination"),
      );
      await host(tester, "one");
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("viewer-album-action")));
      await tester.pumpAndSettle();
      expect(find.text("已选择：Current"), findsOneWidget);
      expect(find.text("移至相册"), findsOneWidget);
      expect(service.memberships, isEmpty);
      await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(service.memberships.single.albumId, "destination");
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "automatic default failure opens the same command for explicit retry",
    (tester) async {
      await setPreferences(
        const AlbumPreferences(
          askBeforeAdding: false,
          defaultAlbumId: "destination",
        ),
      );
      final result = Completer<LibraryOrganizationReceipt>();
      service.saveMembership = (_) => result.future;
      await host(tester, "one");
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("viewer-album-action")));
      await tester.pump();
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(
        tester
            .widget<IconButton>(find.byKey(const Key("viewer-album-action")))
            .onPressed,
        isNull,
      );
      result.completeError(StateError("lost acknowledgement"));
      await tester.pumpAndSettle();
      expect(find.text("尚未确认保存结果，请重试。"), findsOneWidget);
      expect(find.text("已选择：Destination"), findsOneWidget);
      service.saveMembership = null;
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(service.memberships, hasLength(2));
      expect(service.memberships[1].id, service.memberships[0].id);
      expect(
        service.memberships[1].prepared,
        same(service.memberships[0].prepared),
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
    },
  );

  testWidgets("changing the viewer target retires its pending album proposal", (
    tester,
  ) async {
    final first = Completer<LibraryOrganizationInspection>();
    service.inspectTarget = (target) async => target.locationId == "one"
        ? first.future
        : organizationInspection(false);
    await host(tester, "one");
    await tester.pump();
    await tester.tap(find.byKey(const Key("viewer-album-action")));
    await tester.pump();
    await host(tester, "two");
    await tester.pumpAndSettle();
    first.complete(organizationInspection(false));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key("library-album-picker")), findsNothing);
    expect(service.memberships, isEmpty);
    await tester.tap(find.byKey(const Key("viewer-album-action")));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key("library-album-picker")), findsOneWidget);
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(service.memberships, isEmpty);
    expect(tester.takeException(), isNull);
  });
}
