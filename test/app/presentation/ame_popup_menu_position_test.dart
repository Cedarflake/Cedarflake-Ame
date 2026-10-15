import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final scenario in [
    (height: 600.0, textScale: 1.0, itemCount: 2),
    (height: 600.0, textScale: 3.0, itemCount: 2),
    (height: 240.0, textScale: 1.0, itemCount: 20),
  ]) {
    testWidgets("bottom menu stays above its anchor: $scenario", (
      tester,
    ) async {
      tester.view.physicalSize = Size(800, scenario.height);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      var selected = -1;
      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(scenario.textScale)),
            child: child!,
          ),
          home: Scaffold(
            body: Align(
              alignment: Alignment.bottomCenter,
              child: AmePopupMenuButton<int>(
                openAboveAnchor: true,
                labels: [
                  for (var i = 0; i < scenario.itemCount; i++) "Item $i",
                ],
                items: [
                  for (var i = 0; i < scenario.itemCount; i++)
                    PopupMenuItem(
                      value: i,
                      child: AmeMenuItemContent(
                        icon: Icons.copy,
                        label: "Item $i",
                      ),
                    ),
                ],
                onSelected: (value) => selected = value,
                builder: (context, open) => IconButton(
                  key: const Key("anchor"),
                  onPressed: open,
                  icon: const Icon(Icons.more_horiz),
                ),
              ),
            ),
          ),
        ),
      );
      final anchor = find.byKey(const Key("anchor"));
      final anchorRect = tester.getRect(anchor);
      await tester.tap(anchor);
      await tester.pumpAndSettle();
      final menu = find
          .ancestor(of: find.text("Item 0"), matching: find.byType(Material))
          .first;
      final menuRect = tester.getRect(menu);
      expect(menuRect.bottom, lessThanOrEqualTo(anchorRect.top - 4));
      expect(menuRect.top, greaterThanOrEqualTo(8));
      expect(menuRect.overlaps(anchorRect), isFalse);
      final last = scenario.itemCount - 1;
      await tester.ensureVisible(find.text("Item $last"));
      await tester.pumpAndSettle();
      await tester.tap(find.text("Item $last"));
      await tester.pumpAndSettle();
      expect(selected, last);
      expect(tester.takeException(), isNull);
    });
  }
}
