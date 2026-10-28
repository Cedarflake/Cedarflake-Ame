import "dart:ui" show CheckedState;

import "package:cedarflake_ame/app/presentation/ame_selection_checkbox.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  testWidgets("the compact symbol retains pointer and keyboard selection", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      var selected = false;
      var changes = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: StatefulBuilder(
                builder: (context, setState) => AmeSelectionCheckbox(
                  value: selected,
                  markExtent: 28,
                  semanticLabel: "选择图片",
                  onChanged: (value) => setState(() {
                    selected = value!;
                    changes++;
                  }),
                ),
              ),
            ),
          ),
        ),
      );
      final control = find.byType(AmeSelectionCheckbox);
      final checkbox = find.byType(Checkbox);
      final rect = tester.getRect(control);
      expect(rect.size, const Size.square(40));
      expect(tester.getRect(checkbox), rect);
      await tester.tapAt(Offset(rect.left + 2, rect.center.dy));
      await tester.pumpAndSettle();
      expect(selected, isTrue);
      expect(changes, 1);
      expect(find.byIcon(Symbols.check_rounded), findsOneWidget);
      expect(
        tester
            .getSemantics(checkbox)
            .getSemanticsData()
            .flagsCollection
            .isChecked,
        CheckedState.isTrue,
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyEvent(LogicalKeyboardKey.space);
      await tester.pumpAndSettle();
      expect(selected, isFalse);
      expect(changes, 2);
      expect(find.byIcon(Symbols.check_rounded), findsNothing);
      expect(tester.takeException(), isNull);
    } finally {
      semantics.dispose();
    }
  });
}
