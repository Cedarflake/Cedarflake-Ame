import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter/semantics.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
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

  testWidgets("keeps Flutter 3.44 anchored menus non-animated and clickable", (
    tester,
  ) async {
    var activationCount = 0;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmeMenuAnchor(
            menuChildren: [
              MenuItemButton(
                onPressed: () => activationCount += 1,
                child: const Text("打开"),
              ),
            ],
            builder: (context, controller, child) => IconButton(
              key: const Key("non-animated-menu-button"),
              onPressed: () => toggleAmeMenu(controller),
              icon: const Icon(Symbols.more_horiz_rounded),
            ),
          ),
        ),
      ),
    );

    expect(
      tester.widget<MenuAnchor>(find.byType(MenuAnchor)).animated,
      isFalse,
    );

    await tester.tap(find.byKey(const Key("non-animated-menu-button")));
    await tester.pump();
    final item = find.ancestor(
      of: find.text("打开"),
      matching: find.byType(MenuItemButton),
    );
    expect(item, findsOneWidget);
    await tester.tapAt(tester.getCenter(item));
    await tester.pump();
    expect(activationCount, 1);
  });

  testWidgets("keeps non-animated anchored menus keyboard navigable", (
    tester,
  ) async {
    final anchorFocusNode = FocusNode(debugLabel: "menu anchor");
    final firstItemFocusNode = FocusNode(debugLabel: "first menu item");
    final secondItemFocusNode = FocusNode(debugLabel: "second menu item");
    addTearDown(anchorFocusNode.dispose);
    addTearDown(firstItemFocusNode.dispose);
    addTearDown(secondItemFocusNode.dispose);
    var activationCount = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmeMenuAnchor(
            childFocusNode: anchorFocusNode,
            menuChildren: [
              MenuItemButton(
                focusNode: firstItemFocusNode,
                onPressed: () {},
                child: const Text("第一项"),
              ),
              MenuItemButton(
                focusNode: secondItemFocusNode,
                onPressed: () => activationCount += 1,
                child: const Text("第二项"),
              ),
            ],
            builder: (context, controller, child) => IconButton(
              key: const Key("keyboard-menu-button"),
              focusNode: anchorFocusNode,
              onPressed: () => toggleAmeMenu(controller),
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
    await tester.pump();
    expect(find.text("第一项"), findsOneWidget);
    expect(find.text("第二项"), findsOneWidget);
    expect(anchorFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(firstItemFocusNode.hasFocus, isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(secondItemFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(activationCount, 1);
    expect(find.text("第一项"), findsNothing);
    expect(anchorFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pump();
    expect(find.text("第一项"), findsOneWidget);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(firstItemFocusNode.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();
    expect(find.text("第一项"), findsNothing);
    expect(anchorFocusNode.hasFocus, isTrue);
  });

  testWidgets("isolates menu and tooltip overlay traversal semantics", (
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
                AmeMenuAnchor(
                  menuChildren: const [
                    MenuItemButton(onPressed: null, child: Text("菜单项")),
                  ],
                  builder: (context, controller, child) => IconButton(
                    key: const Key("menu-with-tooltip"),
                    tooltip: "更多",
                    onPressed: () => toggleAmeMenu(controller),
                    icon: const Icon(Symbols.more_horiz_rounded),
                  ),
                ),
              ],
            ),
          ),
        ),
      );

      final traversalBoundaries = find.ancestor(
        of: find.byKey(const Key("menu-with-tooltip")),
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is Semantics &&
              widget.container &&
              widget.explicitChildNodes,
        ),
      );
      expect(traversalBoundaries, findsNWidgets(2));

      var semanticsRoot = tester.getSemantics(
        find.byKey(const Key("menu-with-tooltip")),
      );
      var parent = semanticsRoot.parent;
      while (parent != null) {
        semanticsRoot = parent;
        parent = semanticsRoot.parent;
      }
      expect(_countTraversalParents(semanticsRoot), 2);
    } finally {
      semanticsHandle.dispose();
    }
  });

  testWidgets("toggles anchored menus from the same button", (tester) async {
    final controller = MenuController();
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmeMenuAnchor(
            controller: controller,
            menuChildren: const [
              MenuItemButton(onPressed: null, child: Text("菜单项")),
            ],
            builder: (context, controller, child) => IconButton(
              key: const Key("anchored-menu-button"),
              onPressed: () => toggleAmeMenu(controller),
              icon: const Icon(Symbols.more_horiz_rounded),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("anchored-menu-button")));
    await tester.pumpAndSettle();
    expect(find.text("菜单项"), findsOneWidget);

    await tester.tap(find.byKey(const Key("anchored-menu-button")));
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

int _countTraversalParents(SemanticsNode node) {
  var count = node.getSemanticsData().traversalParentIdentifier == null ? 0 : 1;
  node.visitChildren((child) {
    count += _countTraversalParents(child);
    return true;
  });
  return count;
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
