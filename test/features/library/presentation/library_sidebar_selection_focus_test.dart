import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_selection_icon.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_selection_region.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  testWidgets(
    "pointer selection clears row focus while keyboard and Esc work",
    (tester) async {
      final rowFocus = FocusNode();
      final semantics = tester.ensureSemantics();
      debugDefaultTargetPlatformOverride = TargetPlatform.windows;
      var selected = false;
      try {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: StatefulBuilder(
                builder: (context, setState) => LibrarySidebarSelectionRegion(
                  onClear: selected
                      ? () => setState(() => selected = false)
                      : null,
                  child: Align(
                    alignment: Alignment.topLeft,
                    child: SizedBox(
                      width: 300,
                      child: ListTile(
                        focusNode: rowFocus,
                        onTap: () => setState(() => selected = !selected),
                        leading: LibrarySidebarSelectionIcon(
                          icon: Symbols.folder_rounded,
                          name: "Folder",
                          selected: selected,
                          onPressed: () => setState(() => selected = !selected),
                        ),
                        title: const Text("Folder"),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        final icon = find.byType(LibrarySidebarSelectionIcon);
        final selectionSemantics = tester.getSemantics(icon).getSemanticsData();
        expect(selectionSemantics.label, isEmpty);
        expect(selectionSemantics.tooltip, "选择Folder");
        final pointer = await tester.createGesture(
          kind: PointerDeviceKind.mouse,
        );
        await pointer.addPointer(location: const Offset(700, 500));
        await pointer.moveTo(tester.getCenter(icon));
        await pointer.down(tester.getCenter(icon));
        await pointer.up();
        await pointer.moveTo(const Offset(700, 500));
        await tester.pumpAndSettle();
        expect(selected, isTrue);
        expect(rowFocus.hasFocus, isFalse);
        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(selected, isFalse);
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pumpAndSettle();
        expect(rowFocus.hasFocus, isTrue);
        await tester.sendKeyEvent(LogicalKeyboardKey.space);
        await tester.pumpAndSettle();
        expect(selected, isTrue);
        expect(rowFocus.hasFocus, isTrue);
        await pointer.down(tester.getCenter(icon));
        await pointer.up();
        await pointer.moveTo(const Offset(700, 500));
        await tester.pumpAndSettle();
        expect(selected, isFalse);
        expect(rowFocus.hasFocus, isFalse);
        await pointer.removePointer();
        expect(tester.takeException(), isNull);
      } finally {
        await tester.pumpWidget(const SizedBox.shrink());
        rowFocus.dispose();
        semantics.dispose();
        debugDefaultTargetPlatformOverride = null;
      }
    },
  );
}
