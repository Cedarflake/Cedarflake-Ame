import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/library_screen_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  testWidgets("reveals a checkbox on hover and keeps selection visible", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(
      totalItems: 1,
      assetWidth: 1,
      assetHeight: 1000,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final tile = find.byKey(const ValueKey("location-1"));
    final checkbox = find.byKey(const ValueKey("select-location-1"));
    expect(checkbox, findsNothing);

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(tester.getCenter(tile));
    await tester.pump();
    expect(checkbox, findsOneWidget);
    final tileRect = tester.getRect(tile);
    final checkboxRect = tester.getRect(checkbox);
    expect(checkboxRect.left, greaterThanOrEqualTo(tileRect.left));
    expect(checkboxRect.right, lessThanOrEqualTo(tileRect.right));
    expect(checkboxRect.top, greaterThanOrEqualTo(tileRect.top));
    expect(checkboxRect.bottom, lessThanOrEqualTo(tileRect.bottom));

    await tester.tap(checkbox);
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsOneWidget);
    expect(find.text("已选择 1 个项目"), findsOneWidget);
    expect(find.text("查看"), findsNothing);

    await mouse.moveTo(Offset.zero);
    await tester.pump();
    expect(checkbox, findsOneWidget);

    await tester.tap(checkbox);
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);
    expect(find.text("已选择 1 个项目"), findsNothing);
    expect(find.byKey(const Key("library-browsing-toolbar")), findsOneWidget);
  });

  testWidgets(
    "opens the photo context menu with organization actions on secondary click",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryOrganizationProvider.overrideWithValue(
              OrganizationServiceFixture(),
            ),
            initialLibraryStateProvider.overrideWithValue(
              populatedScreenState(totalItems: 1),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      await tester.tap(
        find.byKey(const ValueKey("location-1")),
        buttons: kSecondaryMouseButton,
      );
      await tester.pumpAndSettle();

      expect(find.text("打开"), findsOneWidget);
      expect(find.text("查看信息"), findsOneWidget);
      expect(find.text("复制路径"), findsOneWidget);
      expect(find.text("在文件资源管理器中打开"), findsOneWidget);
      expect(find.text("删除"), findsNothing);
      expect(find.text("加入收藏夹"), findsOneWidget);
      expect(find.text("加入相册"), findsOneWidget);
      expect(find.byType(AmeMenuItemContent), findsNWidgets(6));
    },
  );

  testWidgets("select all covers the complete query instead of loaded tiles", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            populatedScreenState(totalItems: 79013),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    expect(find.text("不选择任何项目"), findsNothing);
    final selectAllMenuItem = find.ancestor(
      of: find.text("全选"),
      matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
    );
    final shortcutFinder = find.text("Ctrl+A");
    final shortcut = tester.widget<Text>(shortcutFinder);
    expect(shortcut.softWrap, isFalse);
    expect(shortcut.overflow, isNull);
    expect(
      tester.getRect(shortcutFinder).right,
      lessThanOrEqualTo(tester.getRect(selectAllMenuItem).right),
    );
    expect(
      tester.view.physicalSize.width - tester.getRect(selectAllMenuItem).right,
      closeTo(AmeMenuMetrics.viewportPadding, 1),
    );
    final moreButtonCenter = tester.getCenter(
      find.byKey(const Key("library-more-menu")),
    );
    await tester.tapAt(moreButtonCenter);
    await tester.pumpAndSettle();
    expect(find.text("全选"), findsNothing);

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("全选"));
    await tester.pumpAndSettle();

    expect(find.text("已选择 79013 个项目"), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-selection")), findsOneWidget);
  });
}
