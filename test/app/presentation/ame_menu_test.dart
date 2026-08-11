import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
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

  testWidgets("keeps shortcuts inside constrained menu rows", (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: const Scaffold(
          body: Center(
            child: SizedBox(
              key: Key("constrained-menu-row"),
              width: 123,
              child: AmeMenuItemContent(
                icon: Symbols.select_all_rounded,
                label: "全选",
                shortcut: "Ctrl+A",
              ),
            ),
          ),
        ),
      ),
    );

    final rowRect = tester.getRect(
      find.byKey(const Key("constrained-menu-row")),
    );
    final shortcutRect = tester.getRect(find.text("Ctrl+A"));
    expect(shortcutRect.right, lessThanOrEqualTo(rowRect.right));
    expect(shortcutRect.left, greaterThan(rowRect.left));
  });

  testWidgets("enables native animation for anchored menus", (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: const Scaffold(
          body: AmeMenuAnchor(
            menuChildren: [MenuItemButton(onPressed: null, child: Text("打开"))],
            child: SizedBox(width: 48, height: 48),
          ),
        ),
      ),
    );

    expect(tester.widget<MenuAnchor>(find.byType(MenuAnchor)).animated, isTrue);
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
