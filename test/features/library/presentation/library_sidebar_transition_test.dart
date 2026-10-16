import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_transition.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "sidebar and gallery move together without intermediate gallery reflows",
    (tester) async {
      final expanded = ValueNotifier(true);
      addTearDown(expanded.dispose);
      final widths = <double>[];
      await tester.pumpWidget(_fixture(expanded, widths));
      final boundary = find.byKey(const Key("library-gallery-transition"));
      expect(tester.getTopLeft(boundary).dx, 260);
      expanded.value = false;
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 70));
      final middle = tester.getTopLeft(boundary).dx;
      expect(middle, greaterThan(0));
      expect(middle, lessThan(260));
      expect(
        tester.getTopLeft(find.byKey(const Key("gallery-content"))).dx,
        middle,
      );
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(boundary).dx, 0);
      expect(find.text("Folders"), findsNothing);
      expect(widths, [540, 800]);
      expanded.value = true;
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 70));
      expect(tester.getTopLeft(boundary).dx, greaterThan(0));
      expect(tester.getTopLeft(boundary).dx, lessThan(260));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(boundary).dx, 260);
      expect(widths, [540, 800, 540]);
      expect(find.text("Folders"), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("a second toggle reverses from the current boundary", (
    tester,
  ) async {
    final expanded = ValueNotifier(true);
    addTearDown(expanded.dispose);
    final widths = <double>[];
    await tester.pumpWidget(_fixture(expanded, widths));
    expanded.value = false;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 70));
    final boundary = find.byKey(const Key("library-gallery-transition"));
    final before = tester.getTopLeft(boundary).dx;
    expanded.value = true;
    await tester.pump();
    expect(tester.getTopLeft(boundary).dx, before);
    await tester.pumpAndSettle();
    expect(tester.getTopLeft(boundary).dx, 260);
    expect(widths, [540, 800, 540]);
    expect(tester.takeException(), isNull);
  });

  testWidgets("reduced motion applies the final layout immediately", (
    tester,
  ) async {
    final expanded = ValueNotifier(true);
    addTearDown(expanded.dispose);
    await tester.pumpWidget(_fixture(expanded, [], reducedMotion: true));
    await tester.pumpAndSettle();
    expect(tester.binding.transientCallbackCount, 0);
    expanded.value = false;
    await tester.pump();
    expect(
      tester.getTopLeft(find.byKey(const Key("library-gallery-transition"))).dx,
      0,
    );
    expect(find.text("Folders"), findsNothing);
    // ExcludeFocus schedules a layout rebuild after its focus microtask. Flush
    // that frame without advancing animation time, then require an idle ticker.
    await tester.pump();
    expect(tester.binding.transientCallbackCount, 0);
    expect(tester.takeException(), isNull);
  });
}

Widget _fixture(
  ValueNotifier<bool> expanded,
  List<double> widths, {
  bool reducedMotion = false,
}) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(context).copyWith(disableAnimations: reducedMotion),
    child: child!,
  ),
  home: Scaffold(
    body: ValueListenableBuilder(
      valueListenable: expanded,
      builder: (_, value, _) => LibrarySidebarTransition(
        expanded: value,
        sidebarWidth: 260,
        sidebar: const Text("Folders"),
        gallery: LayoutBuilder(
          builder: (_, constraints) {
            widths.add(constraints.maxWidth);
            return const SizedBox.expand(key: Key("gallery-content"));
          },
        ),
        resizeHandle: const SizedBox.shrink(),
      ),
    ),
  ),
);
