import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_album_requests.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_editor.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_picker.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late OrganizationLibraryFixture library;
  late ProviderContainer container;
  setUp(() {
    service = OrganizationServiceFixture();
    library = OrganizationLibraryFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(() => library),
      ],
    );
  });
  tearDown(() => container.dispose());

  Future<void> host(
    WidgetTester tester,
    VoidCallback Function(BuildContext) open,
  ) async {
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: open(context),
                child: const Text("Open"),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Open"));
    await tester.pumpAndSettle();
  }

  testWidgets(
    "album editor keeps validation in the dialog and offers the saved album",
    (tester) async {
      LibraryOrganizationReceipt? saved;
      await host(
        tester,
        (context) => () async {
          saved = await showLibraryAlbumEditor(
            context: context,
            kind: LibraryAlbumEditKind.create,
            revision: BigInt.one,
          );
        },
      );
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(service.albumWrites, isEmpty);
      expect(find.text("请输入 1 至 128 个字符的相册名称。"), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key("library-album-name")),
        "Landscape",
      );
      service.saveAlbum = (_) async => throw const LibraryOrganizationFailure(
        "organization_album_name_exists",
        "internal duplicate",
      );
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(find.text("已有同名相册，请换一个名字。"), findsOneWidget);
      expect(find.textContaining("internal duplicate"), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      service.saveAlbum = null;
      await tester.enterText(
        find.byKey(const Key("library-album-name")),
        "Other",
      );
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-album-editor")), findsNothing);
      expect(saved?.album?.name, "Other");
      expect(service.albumWrites[1].id, isNot(service.albumWrites[0].id));
    },
  );

  testWidgets(
    "album picker retains its selected group command until commit and retry",
    (tester) async {
      const album = LibraryOrganizationAlbum("a", "Landscape");
      service.albumItems = [album];
      final committed = Completer<LibraryOrganizationReceipt>();
      service.saveMembership = (_) => committed.future;
      final membership = LibraryAlbumMembershipSession([
        organizationTarget("one"),
      ], isMove: false);
      LibraryAlbumChoice? selected;
      await host(
        tester,
        (context) => () async {
          selected = await showLibraryAlbumPicker(
            context: context,
            actionLabel: "加入相册",
            membership: membership,
          );
        },
      );
      await tester.tap(find.byKey(const ValueKey("album-choice-a")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pump();
      expect(find.text("正在保存"), findsOneWidget);
      expect(selected, isNull);
      expect(find.widgetWithText(TextButton, "取消"), findsNothing);
      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(find.byKey(const Key("library-album-picker")), findsOneWidget);
      committed.completeError(StateError("lost acknowledgement"));
      await tester.pumpAndSettle();
      expect(find.text("尚未确认保存结果，请重试。"), findsOneWidget);
      expect(find.byKey(const Key("library-album-picker")), findsOneWidget);
      service.saveMembership = null;
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(selected?.album?.id, "a");
      expect(service.memberships[1].id, service.memberships[0].id);
    },
  );

  testWidgets(
    "album tray closes at commit without waiting for gallery publication",
    (tester) async {
      const album = LibraryOrganizationAlbum("a", "Landscape");
      service.albumItems = [album];
      final committed = Completer<LibraryOrganizationReceipt>();
      final refreshed = Completer<bool>();
      service.saveMembership = (_) => committed.future;
      library.pendingRefresh = refreshed;
      LibraryAlbumChoice? selected;
      await host(
        tester,
        (context) => () async {
          selected = await showLibraryAlbumPicker(
            context: context,
            actionLabel: "加入相册",
            membership: LibraryAlbumMembershipSession([
              organizationTarget("one"),
            ], isMove: false),
          );
        },
      );
      await tester.tap(find.byKey(const ValueKey("album-choice-a")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pump();
      expect(find.text("正在保存"), findsOneWidget);
      committed.complete(service.receipt(service.memberships.single.id));
      await tester.pumpAndSettle();
      expect(selected?.album, album);
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(library.refreshes, 1);
      expect(refreshed.isCompleted, isFalse);
      refreshed.complete(true);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "bottom tray allows unchecking while keeping a single destination",
    (tester) async {
      const first = LibraryOrganizationAlbum("a", "First");
      const second = LibraryOrganizationAlbum("b", "Second");
      service.albumItems = [first, second];
      await host(
        tester,
        (context) =>
            () => showLibraryAlbumPicker(
              context: context,
              actionLabel: "移至其他相册",
              initialAlbum: first,
              currentAlbum: first,
              clearLabel: "移出相册",
            ),
      );
      expect(find.byType(BottomSheet), findsOneWidget);
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.text("取消"), findsNothing);
      expect(find.textContaining("已选择："), findsNothing);
      final confirm = find.byKey(const Key("library-album-confirm"));
      expect(tester.widget<FilledButton>(confirm).onPressed, isNull);
      await tester.tap(find.byKey(const ValueKey("album-choice-a")));
      await tester.pump();
      expect(find.text("已选择：First"), findsNothing);
      expect(tester.widget<FilledButton>(confirm).onPressed, isNull);
      await tester.tap(find.byKey(const ValueKey("album-choice-b")));
      await tester.pump();
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey("album-choice-b")),
            )
            .value,
        isTrue,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey("album-choice-a")),
            )
            .value,
        isFalse,
      );
      expect(tester.widget<FilledButton>(confirm).onPressed, isNotNull);
      await tester.tap(find.byKey(const ValueKey("album-choice-a")));
      await tester.pump();
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey("album-choice-b")),
            )
            .value,
        isFalse,
      );
      expect(tester.widget<FilledButton>(confirm).onPressed, isNull);
    },
  );

  testWidgets("picker pages retain selection and create within the same flow", (
    tester,
  ) async {
    final requested = <String?>[];
    service.readAlbums = (after) async {
      requested.add(after);
      return LibraryOrganizationAlbumPage(
        albums: [
          LibraryOrganizationAlbum(
            after == null ? "a" : "b",
            after == null ? "First" : "Second",
          ),
        ],
        revision: BigInt.one,
        total: 2,
        nextAlbumId: after == null ? "a" : null,
      );
    };
    LibraryAlbumChoice? selected;
    await host(
      tester,
      (context) => () async {
        selected = await showLibraryAlbumPicker(
          context: context,
          actionLabel: "选择",
        );
      },
    );
    await tester.tap(find.byKey(const ValueKey("album-choice-a")));
    await tester.pump();
    await tester.tap(find.text("下一页"));
    await tester.pumpAndSettle();
    expect(find.text("Second"), findsOneWidget);
    expect(find.textContaining("已选择："), findsNothing);
    expect(requested, [null, "a"]);
    await tester.tap(find.text("上一页"));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey("album-choice-a")),
          )
          .value,
      isTrue,
    );
    await tester.tap(find.byKey(const Key("library-album-create")));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key("library-album-name")),
      "New album",
    );
    await tester.tap(find.byKey(const Key("library-album-save")));
    await tester.pumpAndSettle();
    expect(find.textContaining("已选择："), findsNothing);
    await tester.tap(find.byKey(const Key("library-album-confirm")));
    await tester.pumpAndSettle();
    expect(selected?.album?.name, "New album");
    expect(service.memberships, isEmpty);
  });

  testWidgets(
    "cancel leaves membership unchanged and removal uses the full selection",
    (tester) async {
      const album = LibraryOrganizationAlbum("a", "Current");
      service.albumItems = [album];
      Future<void> open() => host(
        tester,
        (context) =>
            () => showLibraryAlbumPicker(
              context: context,
              actionLabel: "移至相册",
              clearLabel: "移出相册",
              initialAlbum: album,
              membership: LibraryAlbumMembershipSession([
                organizationTarget("one"),
                organizationTarget("two"),
              ], isMove: true),
            ),
      );
      await open();
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(service.memberships, isEmpty);
      await open();
      await tester.tap(find.text("移出相册"));
      await tester.pumpAndSettle();
      expect(service.memberships.single.albumId, isNull);
      expect(service.preparedTargets.single, [
        organizationTarget("one"),
        organizationTarget("two"),
      ]);
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
    },
  );
}
