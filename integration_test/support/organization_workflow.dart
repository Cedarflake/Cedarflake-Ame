import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyOrganizationWorkflow(
  WidgetTester tester,
  ProviderContainer container,
) async {
  final workflow = _OrganizationWorkflow(tester, container);
  await workflow.verifyKeyboardSelection();
  await workflow.verifyViewerChanges();
  await workflow.verifyAlbumManagement();
  await workflow.verifyCreationUndo();
  expect(tester.takeException(), isNull);
  debugPrint("AME_ORGANIZATION_NATIVE_WORKFLOW_PASSED");
}

class _OrganizationWorkflow {
  _OrganizationWorkflow(this.tester, this.container);

  final WidgetTester tester;
  final ProviderContainer container;
  final service = const RustLibraryOrganization();
  late LibraryOrganizationTarget target;
  late LibraryOrganizationAlbum album;

  Future<void> verifyKeyboardSelection() async {
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    try {
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
    } finally {
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    }
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsOneWidget);
    expect(find.text("已选择 2 个项目"), findsOneWidget);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);
  }

  Future<void> verifyViewerChanges() async {
    final tile = find.byType(LibraryPhotoTile).hitTestable().first;
    final asset = tester.widget<LibraryPhotoTile>(tile).asset;
    target = LibraryOrganizationTarget.fromAsset(asset);
    await tester.tapAt(tester.getRect(tile).topLeft + const Offset(16, 16));
    await waitFor(() => favoriteButton?.onPressed != null);

    await tapKey("viewer-favorite");
    await waitFor(() => favoriteButton?.isSelected == true);
    await undo(viewer: true);
    await waitFor(() => favoriteButton?.isSelected == false);
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.pending,
    );
    expect(
      container
          .read(ameNotificationControllerProvider)
          .history
          .where(
            (entry) => entry.dedupeKey?.startsWith("organization-") == true,
          ),
      isEmpty,
    );

    await openPicker();
    await tapKey("library-album-create");
    await waitFor(
      () => find.byKey(const Key("library-album-name")).evaluate().isNotEmpty,
    );
    await tapKey("library-album-save");
    expect(find.text("请输入 1 至 128 个字符的相册名称。"), findsOneWidget);
    await tester.enterText(
      find.byKey(const Key("library-album-name")),
      "Integration album",
    );
    await tapKey("library-album-save");
    await waitFor(
      () => find.byKey(const Key("library-album-editor")).evaluate().isEmpty,
    );
    album = (await service.albums()).albums.singleWhere(
      (value) => value.name == "Integration album",
    );
    await tapKey("library-album-confirm");
    await waitFor(
      () => find.byKey(const Key("library-album-picker")).evaluate().isEmpty,
    );
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.organized,
      albumId: album.id,
    );
    expect(currentTarget, target);
    await undo(viewer: true);
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.pending,
    );
    expect(currentTarget, target);

    await openPicker();
    await tapKey("album-choice-${album.id}");
    await tapKey("library-album-confirm");
    await waitFor(
      () => find.byKey(const Key("library-album-picker")).evaluate().isEmpty,
    );
    await openPicker();
    await tester.tap(find.text("移出相册"));
    await tester.pump();
    await waitFor(
      () => find.byKey(const Key("library-album-picker")).evaluate().isEmpty,
    );
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.ignored,
    );
    await undo(viewer: true);
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.organized,
      albumId: album.id,
    );
    expect(currentTarget, target);
    await tapKey("viewer-back-button");
    await waitFor(() => find.byType(LibraryViewerImage).evaluate().isEmpty);
  }

  Future<void> verifyAlbumManagement() async {
    await tapKey("workbench-albums");
    await waitFor(() => albumTile.evaluate().isNotEmpty);
    await tester.tap(albumTile);
    await tester.pumpAndSettle();
    await openAlbumMenu();
    await tester.tap(find.text("重命名"));
    await tester.pumpAndSettle();
    final input = find.byKey(const Key("library-album-name"));
    await tester.enterText(input, "Integration renamed");
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    try {
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
    } finally {
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    }
    await tester.pump();
    final editingValue = tester.widget<TextField>(input).controller!.value;
    expect(editingValue.selection.start, 0);
    expect(editingValue.selection.end, editingValue.text.length);
    expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);

    await service.createAlbum(
      "native-concurrent-album-${DateTime.now().microsecondsSinceEpoch}",
      await service.revision(),
      "Concurrent album",
    );
    await tapKey("library-album-save");
    await waitFor(() => find.text("图片或相册信息已更新，请检查后重试。").evaluate().isNotEmpty);
    expect((await service.findAlbum(album.id))?.name, "Integration album");
    expect(find.byKey(const Key("library-album-editor")), findsOneWidget);
    await tapKey("library-album-save");
    await waitFor(
      () => find.byKey(const Key("library-album-editor")).evaluate().isEmpty,
    );
    expect((await service.findAlbum(album.id))?.name, "Integration renamed");
    await undo(viewer: false);
    expect((await service.findAlbum(album.id))?.name, "Integration album");

    await openAlbumMenu();
    await tester.tap(find.text("删除相册"));
    await tester.pumpAndSettle();
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(await service.findAlbum(album.id), isNotNull);
    await openAlbumMenu();
    await tester.tap(find.text("删除相册"));
    await tester.pumpAndSettle();
    await tapKey("library-album-save");
    await waitFor(
      () => find.byKey(const Key("library-album-editor")).evaluate().isEmpty,
    );
    expect(await service.findAlbum(album.id), isNull);
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.ignored,
    );
    await undo(viewer: false);
    expect(await service.findAlbum(album.id), isNotNull);
    await expectDecision(
      favorite: false,
      disposition: LibraryOrganizationDisposition.organized,
      albumId: album.id,
    );
    await waitFor(() => albumTile.evaluate().isNotEmpty);
  }

  Future<void> verifyCreationUndo() async {
    for (final leaveAlbum in [false, true]) {
      await tapKey("library-sidebar-create-album");
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key("library-album-name")),
        "Temporary album",
      );
      await tapKey("library-album-save");
      await waitFor(
        () => find.byKey(const Key("library-album-editor")).evaluate().isEmpty,
      );
      final temporary = (await service.albums()).albums.singleWhere(
        (value) => value.name == "Temporary album",
      );
      final tile = find.byKey(
        ValueKey("library-sidebar-album-${temporary.id}"),
      );
      await waitFor(() => tile.evaluate().isNotEmpty);
      await tester.tap(tile);
      await tester.pumpAndSettle();
      if (leaveAlbum) {
        await tapKey("workbench-gallery");
        await tester.pumpAndSettle();
      }
      await undo(viewer: false);
      expect(await service.findAlbum(temporary.id), isNull);
      if (leaveAlbum) {
        await tapKey("workbench-albums");
        await tester.pumpAndSettle();
      }
      expect(tile, findsNothing);
      expect(find.text("选择相册"), findsOneWidget);
      expect(find.text("图库尚未刷新，请重试。"), findsNothing);
    }
  }

  IconButton? get favoriteButton {
    final finder = find.byKey(const Key("viewer-favorite"));
    return finder.evaluate().isEmpty ? null : tester.widget<IconButton>(finder);
  }

  LibraryOrganizationTarget get currentTarget =>
      LibraryOrganizationTarget.fromAsset(
        tester
            .widget<LibraryViewerImage>(find.byType(LibraryViewerImage))
            .asset,
      );

  Finder get albumTile =>
      find.byKey(ValueKey("library-sidebar-album-${album.id}"));

  Future<void> openPicker() async {
    await waitFor(
      () =>
          tester
              .widget<IconButton>(find.byKey(const Key("viewer-album-action")))
              .onPressed !=
          null,
    );
    await tapKey("viewer-album-action");
    await waitFor(
      () => find.byKey(const Key("library-album-picker")).evaluate().isNotEmpty,
    );
    await tester.pumpAndSettle();
  }

  Future<void> openAlbumMenu() async {
    final menu = find.byKey(ValueKey("library-sidebar-album-menu-${album.id}"));
    await waitFor(
      () =>
          menu.evaluate().isNotEmpty &&
          tester.widget<IconButton>(menu).onPressed != null,
    );
    await tester.tap(menu);
    await tester.pumpAndSettle();
  }

  Future<void> undo({required bool viewer}) async {
    final key = viewer
        ? "viewer-organization-action"
        : "notification-primary-action";
    await waitFor(
      () => find.byKey(Key(key)).hitTestable().evaluate().isNotEmpty,
    );
    expect(
      find.descendant(of: find.byKey(Key(key)), matching: find.text("撤销")),
      findsOneWidget,
    );
    await tapKey(key);
    await waitFor(
      () => !container.read(libraryOrganizationActionsProvider).saving,
    );
    await tester.pumpAndSettle();
    expect(find.text("无法撤销"), findsNothing);
    expect(find.text("未能保存更改"), findsNothing);
  }

  Future<void> expectDecision({
    required bool favorite,
    required LibraryOrganizationDisposition disposition,
    String? albumId,
  }) async {
    final decision = (await service.inspect([target])).decisions.single;
    expect(decision.favorite, favorite);
    expect(decision.disposition, disposition);
    expect(decision.album?.id, albumId);
  }

  Future<void> tapKey(String value) async {
    final finder = find.byKey(Key(value));
    await tester.ensureVisible(finder);
    await tester.tap(finder);
    await tester.pump();
  }

  Future<void> waitFor(bool Function() condition) async {
    final deadline = DateTime.now().add(const Duration(seconds: 15));
    while (!condition()) {
      if (DateTime.now().isAfter(deadline)) {
        throw TestFailure(
          "The native organization interaction did not complete",
        );
      }
      await tester.pump(const Duration(milliseconds: 25));
    }
    await tester.pump();
  }
}
