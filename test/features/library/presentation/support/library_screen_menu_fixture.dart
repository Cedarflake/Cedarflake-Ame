import "dart:ui" show CheckedState;

import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void expectSelectedMenuChoice(
  WidgetTester tester, {
  required String label,
  required IconData icon,
}) {
  final item = menuItem(label);
  expect(
    find.descendant(of: item, matching: find.byIcon(icon)),
    findsOneWidget,
  );
  expect(
    find.descendant(of: item, matching: find.byIcon(Symbols.circle_rounded)),
    findsOneWidget,
  );
  final semantics = tester.getSemantics(
    find.byKey(ValueKey("menu-choice-$label")),
  );
  expect(semantics.flagsCollection.isChecked, CheckedState.isTrue);
  expect(semantics.flagsCollection.isInMutuallyExclusiveGroup, isTrue);
}

Finder menuItem(String label) {
  return find.ancestor(
    of: find.text(label).last,
    matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
  );
}

Finder menuChoiceTarget(String label) {
  return find.descendant(of: menuItem(label), matching: find.byType(InkWell));
}

Future<void> selectPopupMenuItem(WidgetTester tester, Finder item) async {
  await tester.tap(item);
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
}

Finder activeMenuItem(WidgetTester tester, String label) {
  final item = activeMenuItemOrNull(tester, label);
  expect(item, isNotNull);
  return item!;
}

Finder? activeMenuItemOrNull(WidgetTester tester, String label) {
  final candidates = find.ancestor(
    of: find.text(label),
    matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
  );
  if (candidates.evaluate().length != 1) {
    return null;
  }
  final offstageAncestors = find
      .ancestor(of: candidates, matching: find.byType(Offstage))
      .evaluate()
      .map((element) => element.widget as Offstage);
  expect(offstageAncestors.every((widget) => !widget.offstage), isTrue);
  final ignoringAncestors = find
      .ancestor(of: candidates, matching: find.byType(IgnorePointer))
      .evaluate()
      .map((element) => element.widget as IgnorePointer);
  expect(ignoringAncestors.every((widget) => !widget.ignoring), isTrue);
  final choiceSemantics = find.descendant(
    of: candidates,
    matching: find.byKey(ValueKey("menu-choice-$label")),
  );
  expect(choiceSemantics, findsOneWidget);
  return candidates;
}
