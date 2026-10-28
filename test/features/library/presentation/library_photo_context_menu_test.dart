import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_context_menu.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  late LibraryPhotoContextMenu menu;
  late LibraryOrganizationTarget target;
  final copied = <LibraryOrganizationTarget>[];
  const album = LibraryOrganizationAlbum("one", "Album");
  const destination = LibraryOrganizationAlbum("two", "Destination");

  setUp(() {
    service = OrganizationServiceFixture()..albumItems = [album, destination];
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
    menu = LibraryPhotoContextMenu();
    target = organizationTarget("one");
    copied.clear();
  });
  tearDown(() {
    menu.dispose();
    container.dispose();
  });

  Future<void> open(WidgetTester tester) async {
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Consumer(
              builder: (context, ref, _) => TextButton(
                onPressed: () {
                  final openingTarget = target;
                  unawaited(
                    menu.open(
                      context: context,
                      ref: ref,
                      position: const RelativeRect.fromLTRB(10, 40, 500, 0),
                      target: openingTarget,
                      isCurrent: () => openingTarget == target,
                      onOpen: () {},
                      onInformation: () {},
                      onCopyPath: () => copied.add(openingTarget),
                      onReveal: () {},
                    ),
                  );
                },
                child: const Text("Menu"),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Menu"));
    await tester.pumpAndSettle();
  }

  void assigned({bool favorite = false, bool conflict = false}) {
    service.inspectTarget = (_) async => LibraryOrganizationInspection(
      revision: BigInt.one,
      decisions: [
        LibraryOrganizationDecision(
          favorite: favorite,
          disposition: LibraryOrganizationDisposition.organized,
          album: conflict ? null : album,
          hasConflict: conflict,
        ),
      ],
    );
  }

  testWidgets("favorite toggles through the existing transient command", (
    tester,
  ) async {
    await open(tester);
    expect(find.text("取消收藏"), findsNothing);
    expect(find.text("移出相册"), findsNothing);
    await tester.tap(find.text("加入收藏夹"));
    await tester.pumpAndSettle();
    expect(service.writes.single.favorite, isTrue);
    expect(service.preparedTargets.single.single, target);
    expect(container.read(ameNotificationControllerProvider).history, isEmpty);
    await open(tester);
    expect(find.text("加入收藏夹"), findsNothing);
    await tester.tap(find.text("取消收藏"));
    await tester.pumpAndSettle();
    expect(service.writes.last.favorite, isFalse);
  });

  testWidgets("album movement opens the shared tray without progress actions", (
    tester,
  ) async {
    assigned();
    await open(tester);
    expect(find.text("加入相册"), findsNothing);
    expect(find.text("移出相册"), findsOneWidget);
    await tester.tap(find.text("移至其他相册"));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key("library-album-picker")), findsOneWidget);
    expect(find.byKey(const Key("library-album-ignore")), findsNothing);
    expect(find.byKey(const Key("library-album-defer")), findsNothing);
    await tester.tap(find.byKey(const ValueKey("album-choice-two")));
    await tester.pump();
    await tester.tap(find.byKey(const Key("library-album-confirm")));
    await tester.pumpAndSettle();
    expect(service.memberships.single.albumId, "two");
    expect(service.preparedTargets.single.single, target);
  });

  testWidgets("direct removal preserves notice retry on a failed save", (
    tester,
  ) async {
    assigned();
    service.saveMembership = (_) async =>
        throw const LibraryOrganizationFailure("busy", "busy");
    await open(tester);
    await tester.tap(find.text("移出相册"));
    await tester.pumpAndSettle();
    expect(service.memberships.single.albumId, isNull);
    expect(
      container.read(ameNotificationControllerProvider).history.single.actionId,
      "organization.retry",
    );
  });

  testWidgets("late inspection and menu callbacks reject a replaced target", (
    tester,
  ) async {
    final inspection = Completer<LibraryOrganizationInspection>();
    service.inspectTarget = (_) => inspection.future;
    await open(tester);
    target = organizationTarget("two");
    inspection.complete(organizationInspection(false));
    await tester.pumpAndSettle();
    expect(find.text("复制路径"), findsNothing);
    service.inspectTarget = null;
    await open(tester);
    target = organizationTarget("three");
    await tester.tap(find.text("复制路径"));
    await tester.pumpAndSettle();
    expect(copied, isEmpty);
    expect(service.writes, isEmpty);
  });

  testWidgets(
    "an inspection failure leaves basic actions and allows reopening",
    (tester) async {
      service.inspectTarget = (_) async =>
          throw const LibraryOrganizationFailure("unavailable", "unavailable");
      await open(tester);
      expect(find.text("加入收藏夹"), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).history.single.title,
        "暂时无法读取相册和收藏",
      );
      await tester.tap(find.text("复制路径"));
      await tester.pumpAndSettle();
      expect(copied, [target]);
      service.inspectTarget = null;
      await open(tester);
      expect(find.text("加入收藏夹"), findsOneWidget);
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isFalse,
      );
    },
  );
}
