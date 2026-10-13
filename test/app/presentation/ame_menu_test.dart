import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  testWidgets("uses portal-free tooltip semantics on Windows", (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        const MaterialApp(
          home: AmeTooltip(
            message: "完整路径",
            child: SizedBox(key: Key("tooltip-child"), width: 24, height: 24),
          ),
        ),
      );

      expect(find.byType(Tooltip), findsNothing);
      expect(
        tester.getSemantics(find.byKey(const Key("tooltip-child"))),
        matchesSemantics(tooltip: "完整路径"),
      );
    } finally {
      semanticsHandle.dispose();
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets("shows and dismisses a portal-free Windows tooltip", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    try {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: Center(
              child: AmeTooltip(
                message: "完整路径",
                waitDuration: Duration.zero,
                child: SizedBox(
                  key: Key("visual-tooltip-child"),
                  width: 24,
                  height: 24,
                ),
              ),
            ),
          ),
        ),
      );

      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: Offset.zero);
      await mouse.moveTo(
        tester.getCenter(find.byKey(const Key("visual-tooltip-child"))),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));

      expect(find.text("完整路径"), findsOneWidget);
      expect(find.byType(Tooltip), findsNothing);

      await mouse.moveTo(Offset.zero);
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pump(const Duration(milliseconds: 150));

      expect(find.text("完整路径"), findsNothing);

      await mouse.moveTo(
        tester.getCenter(find.byKey(const Key("visual-tooltip-child"))),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));

      expect(find.text("完整路径"), findsOneWidget);
      await mouse.removePointer();
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets("shows the portal-free Windows tooltip for keyboard focus", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    final focusNode = FocusNode();
    try {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: AmeTooltip(
                message: "键盘说明",
                child: TextButton(
                  key: const Key("focus-tooltip-child"),
                  focusNode: focusNode,
                  onPressed: () {},
                  child: const Text("操作"),
                ),
              ),
            ),
          ),
        ),
      );

      focusNode.requestFocus();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));

      expect(find.text("键盘说明"), findsOneWidget);
      expect(find.byType(Tooltip), findsNothing);

      focusNode.unfocus();
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pumpAndSettle();

      expect(find.text("键盘说明"), findsNothing);
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      focusNode.dispose();
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets("shows the portal-free Windows tooltip after a touch hold", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    try {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: Center(
              child: AmeTooltip(
                message: "触屏说明",
                child: SizedBox(
                  key: Key("touch-tooltip-child"),
                  width: 48,
                  height: 48,
                ),
              ),
            ),
          ),
        ),
      );

      final touch = await tester.startGesture(
        tester.getCenter(find.byKey(const Key("touch-tooltip-child"))),
        kind: PointerDeviceKind.touch,
      );
      await tester.pump(kLongPressTimeout + const Duration(milliseconds: 1));
      await tester.pump(const Duration(milliseconds: 150));

      expect(find.text("触屏说明"), findsOneWidget);
      expect(find.byType(Tooltip), findsNothing);

      await touch.up();
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pumpAndSettle();

      expect(find.text("触屏说明"), findsNothing);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  test("uses one visual contract for anchored and popup menus", () {
    final theme = buildAmeTheme();
    final menuStyle = _require(theme.menuTheme.style, "menu style");
    final popupStyle = theme.popupMenuTheme;
    const states = <WidgetState>{};

    expect(
      _require(menuStyle.backgroundColor, "menu background").resolve(states),
      popupStyle.color,
    );
    expect(
      _require(menuStyle.shadowColor, "menu shadow").resolve(states),
      popupStyle.shadowColor,
    );
    expect(
      _require(menuStyle.surfaceTintColor, "menu tint").resolve(states),
      popupStyle.surfaceTintColor,
    );
    expect(
      _require(menuStyle.elevation, "menu elevation").resolve(states),
      popupStyle.elevation,
    );
    expect(
      _require(menuStyle.padding, "menu padding").resolve(states),
      popupStyle.menuPadding,
    );
    expect(
      _require(menuStyle.shape, "menu shape").resolve(states),
      popupStyle.shape,
    );

    final buttonStyle = _require(
      theme.menuButtonTheme.style,
      "menu button style",
    );
    expect(
      _require(
        buttonStyle.minimumSize,
        "menu item minimum size",
      ).resolve(states),
      const Size(AmeMenuMetrics.minimumWidth, AmeMenuMetrics.itemHeight),
    );
    expect(
      _require(buttonStyle.padding, "menu item padding").resolve(states),
      AmeMenuMetrics.itemPadding,
    );
    expect(
      _require(buttonStyle.iconSize, "menu item icon size").resolve(states),
      AmeMenuMetrics.iconSize,
    );
  });

  testWidgets("uses shared icon and label geometry", (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: const Scaffold(
          body: Center(
            child: AmeMenuItemContent(
              icon: Symbols.folder_rounded,
              label: "图片",
            ),
          ),
        ),
      ),
    );

    final iconRect = tester.getRect(find.byIcon(Symbols.folder_rounded));
    final labelRect = tester.getRect(find.text("图片"));
    expect(iconRect.size, const Size.square(AmeMenuMetrics.iconSize));
    expect(labelRect.left - iconRect.right, AmeMenuMetrics.iconLabelGap);
  });

  testWidgets("emphasizes selected menu choices without flattening labels", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: const Scaffold(
          body: Column(
            children: [
              AmeMenuItemContent(icon: Symbols.sort_rounded, label: "普通"),
              AmeMenuItemContent(
                icon: Symbols.check_rounded,
                label: "已选择",
                isSelected: true,
              ),
            ],
          ),
        ),
      ),
    );

    expect(
      tester.widget<Text>(find.text("普通")).style?.fontWeight,
      ameFontWeightMedium,
    );
    expect(
      tester.widget<Text>(find.text("已选择")).style?.fontWeight,
      ameFontWeightSemibold,
    );
  });

  testWidgets("keeps shortcuts fully visible at calculated menu width", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Builder(
          builder: (context) {
            final menuWidth = amePopupMenuContentWidth(
              context: context,
              labels: const ["全选"],
              shortcuts: const ["Ctrl+A"],
            );
            return Scaffold(
              body: Center(
                child: SizedBox(
                  key: const Key("calculated-menu-row"),
                  width: menuWidth,
                  child: MenuItemButton(
                    onPressed: () {},
                    child: const AmeMenuItemContent(
                      icon: Symbols.select_all_rounded,
                      label: "全选",
                      shortcut: "Ctrl+A",
                    ),
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );

    final rowRect = tester.getRect(
      find.byKey(const Key("calculated-menu-row")),
    );
    final shortcutFinder = find.text("Ctrl+A");
    final shortcut = tester.widget<Text>(shortcutFinder);
    final shortcutContext = tester.element(shortcutFinder);
    final shortcutPainter = TextPainter(
      text: TextSpan(
        text: shortcut.data,
        style: DefaultTextStyle.of(shortcutContext).style.merge(shortcut.style),
      ),
      textDirection: Directionality.of(shortcutContext),
      textScaler: MediaQuery.textScalerOf(shortcutContext),
      maxLines: 1,
    )..layout();
    final shortcutRect = tester.getRect(shortcutFinder);

    expect(shortcut.softWrap, isFalse);
    expect(shortcutRect.width, closeTo(shortcutPainter.width, 0.01));
    expect(shortcutRect.right, lessThanOrEqualTo(rowRect.right));
    expect(shortcutRect.left, greaterThan(rowRect.left));
    shortcutPainter.dispose();
  });

  testWidgets("includes Material leading icon geometry in calculated width", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Builder(
          builder: (context) {
            return MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(textScaler: const TextScaler.linear(1.5)),
              child: Builder(
                builder: (context) {
                  final menuWidth = amePopupMenuContentWidth(
                    context: context,
                    labels: const ["拍摄日期"],
                    leadingIconWidth:
                        AmeMenuMetrics.selectionIndicatorSlotWidth,
                  );
                  return Scaffold(
                    body: Center(
                      child: SizedBox(
                        key: const Key("leading-icon-menu-row"),
                        width: menuWidth,
                        child: MenuItemButton(
                          onPressed: () {},
                          leadingIcon: const SizedBox(
                            width: AmeMenuMetrics.selectionIndicatorSlotWidth,
                          ),
                          child: const AmeMenuItemContent(
                            icon: Symbols.calendar_month_rounded,
                            label: "拍摄日期",
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            );
          },
        ),
      ),
    );

    _expectTextFits(tester, "拍摄日期");
  });

  testWidgets("animates and activates anchored popup routes", (tester) async {
    var activationCount = 0;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmePopupMenuButton<int>(
            labels: const ["打开"],
            items: const [PopupMenuItem(value: 1, child: Text("打开"))],
            onSelected: (_) => activationCount += 1,
            builder: (context, openMenu) => IconButton(
              key: const Key("animated-menu-button"),
              onPressed: openMenu,
              icon: const Icon(Symbols.more_horiz_rounded),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("animated-menu-button")));
    await tester.pump();
    final item = find.ancestor(
      of: find.text("打开"),
      matching: find.byType(PopupMenuItem<int>),
    );
    expect(item, findsOneWidget);
    final route = ModalRoute.of(tester.element(item));
    expect(
      AmeMenuTransition.popupAnimationStyle,
      isNot(AnimationStyle.noAnimation),
    );
    expect(route?.transitionDuration, AmeMenuTransition.duration);
    expect(route?.reverseTransitionDuration, AmeMenuTransition.duration);
    expect(route?.animation?.isCompleted, isFalse);

    await tester.pump(const Duration(milliseconds: 100));
    expect(route?.animation?.value, greaterThan(0));
    expect(route?.animation?.value, lessThan(1));
    await tester.pumpAndSettle();
    await tester.tapAt(tester.getCenter(item));
    await tester.pumpAndSettle();
    expect(activationCount, 1);
  });

  testWidgets("uses the same transition contract for direct popup routes", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              key: const Key("popup-menu-button"),
              onPressed: () {
                unawaited(
                  showAmePopupMenu<void>(
                    context: context,
                    position: const RelativeRect.fromLTRB(24, 24, 24, 24),
                    labels: const ["打开"],
                    items: const [PopupMenuItem<void>(child: Text("打开"))],
                  ),
                );
              },
              child: const Text("菜单"),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("popup-menu-button")));
    await tester.pump();

    final route = ModalRoute.of(tester.element(find.text("打开")));
    expect(route?.transitionDuration, AmeMenuTransition.duration);
    expect(route?.reverseTransitionDuration, AmeMenuTransition.duration);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
  });

  testWidgets("keeps popup-route menus keyboard navigable", (tester) async {
    final anchorFocusNode = FocusNode(debugLabel: "menu anchor");
    addTearDown(anchorFocusNode.dispose);
    int? selected;

    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmePopupMenuButton<int>(
            labels: const ["第一项", "第二项"],
            items: const [
              PopupMenuItem(value: 1, child: Text("第一项")),
              PopupMenuItem(value: 2, child: Text("第二项")),
            ],
            onSelected: (value) => selected = value,
            builder: (context, openMenu) => IconButton(
              key: const Key("keyboard-menu-button"),
              focusNode: anchorFocusNode,
              onPressed: openMenu,
              icon: const Icon(Symbols.more_horiz_rounded),
            ),
          ),
        ),
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(anchorFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(find.text("第一项"), findsOneWidget);
    expect(find.text("第二项"), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(selected, 2);
    expect(find.text("第一项"), findsNothing);
    expect(anchorFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(find.text("第一项"), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(find.text("第一项"), findsNothing);
    expect(anchorFocusNode.hasFocus, isTrue);
  });

  testWidgets("keeps MenuAnchor portals out of anchored popup menus", (
    tester,
  ) async {
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: ListView(
              children: [
                AmePopupMenuButton<int>(
                  labels: const ["菜单项"],
                  items: const [PopupMenuItem(value: 1, child: Text("菜单项"))],
                  onSelected: (_) {},
                  builder: (context, openMenu) => IconButton(
                    key: const Key("menu-with-tooltip"),
                    tooltip: "更多",
                    onPressed: openMenu,
                    icon: const Icon(Symbols.more_horiz_rounded),
                  ),
                ),
              ],
            ),
          ),
        ),
      );

      expect(find.byType(MenuAnchor), findsNothing);
      await tester.tap(find.byKey(const Key("menu-with-tooltip")));
      await tester.pumpAndSettle();
      expect(find.text("菜单项"), findsOneWidget);
      expect(find.byType(MenuAnchor), findsNothing);
    } finally {
      semanticsHandle.dispose();
    }
  });

  testWidgets("dismisses an anchored popup from its button position", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmePopupMenuButton<int>(
            labels: const ["菜单项"],
            items: const [PopupMenuItem(value: 1, child: Text("菜单项"))],
            onSelected: (_) {},
            builder: (context, openMenu) => IconButton(
              key: const Key("popup-anchor-button"),
              onPressed: openMenu,
              icon: const Icon(Symbols.more_horiz_rounded),
            ),
          ),
        ),
      ),
    );

    final button = find.byKey(const Key("popup-anchor-button"));
    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(find.text("菜单项"), findsOneWidget);

    await tester.tapAt(tester.getCenter(button));
    await tester.pumpAndSettle();
    expect(find.text("菜单项"), findsNothing);
  });

  testWidgets("sizes popup menus from their longest label", (tester) async {
    late double shortWidth;
    late double shortcutWidth;
    late double longWidth;
    late double cappedWidth;

    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Builder(
          builder: (context) {
            shortWidth = amePopupMenuContentWidth(
              context: context,
              labels: const ["打开"],
            );
            shortcutWidth = amePopupMenuContentWidth(
              context: context,
              labels: const ["全选"],
              shortcuts: const ["Ctrl+A"],
            );
            longWidth = amePopupMenuContentWidth(
              context: context,
              labels: const ["在文件资源管理器中打开"],
            );
            cappedWidth = amePopupMenuContentWidth(
              context: context,
              labels: const ["这是一个用于验证菜单最大宽度限制且不会无限扩张的非常长的菜单项目"],
            );
            return const SizedBox();
          },
        ),
      ),
    );

    expect(shortWidth, AmeMenuMetrics.minimumWidth);
    expect(shortcutWidth, greaterThan(shortWidth));
    expect(longWidth, greaterThan(shortWidth));
    expect(cappedWidth, AmeMenuMetrics.maximumWidth);
  });
}

T _require<T>(T? value, String description) {
  if (value == null) {
    throw TestFailure("Missing $description");
  }
  return value;
}

void _expectTextFits(WidgetTester tester, String label) {
  final finder = find.text(label);
  final text = tester.widget<Text>(finder);
  final context = tester.element(finder);
  final painter = TextPainter(
    text: TextSpan(
      text: text.data,
      style: DefaultTextStyle.of(context).style.merge(text.style),
    ),
    textDirection: Directionality.of(context),
    textScaler: MediaQuery.textScalerOf(context),
    maxLines: 1,
  )..layout();
  expect(tester.getSize(finder).width, greaterThanOrEqualTo(painter.width));
  painter.dispose();
}
