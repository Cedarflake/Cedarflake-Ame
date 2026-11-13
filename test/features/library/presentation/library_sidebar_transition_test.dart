import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_content.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_transition.dart";
import "package:flutter/rendering.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "content motion preserves clipping and hit targets without relayout",
    (tester) async {
      final progress = AnimationController(vsync: tester);
      addTearDown(progress.dispose);
      var layouts = 0;
      var taps = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 400,
              child: _LayoutProbe(
                onLayout: () => layouts++,
                child: LibrarySidebarMotion(
                  progress: progress,
                  sidebarWidth: 260,
                  expanded: false,
                  child: LibrarySidebarContent(
                    alignment: Alignment.topCenter,
                    child: GestureDetector(
                      onTap: () => taps++,
                      child: const SizedBox(
                        key: Key("moving-content"),
                        height: 200,
                        child: ColoredBox(color: Colors.blue),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final initialLayouts = layouts;
      progress.value = 0.5;
      await tester.pump();
      expect(layouts, initialLayouts);
      expect(tester.getTopLeft(find.byKey(const Key("moving-content"))).dx, 65);
      await tester.tapAt(const Offset(100, 50));
      expect(taps, 0);
      await tester.tapAt(const Offset(150, 50));
      expect(taps, 1);
      progress.value = 0;
      await tester.pump();
      expect(layouts, initialLayouts);
      expect(tester.getTopLeft(find.byKey(const Key("moving-content"))).dx, 0);
    },
  );

  testWidgets(
    "sidebar boundary animates around a stationary right gallery edge",
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
        tester.getTopRight(find.byKey(const Key("gallery-content"))).dx,
        800,
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
    expect(
      tester.getTopRight(find.byKey(const Key("gallery-content"))).dx,
      800,
    );
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

class _LayoutProbe extends SingleChildRenderObjectWidget {
  const _LayoutProbe({required this.onLayout, required super.child});

  final VoidCallback onLayout;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _LayoutProbeRenderObject(onLayout);
}

class _LayoutProbeRenderObject extends RenderProxyBox {
  _LayoutProbeRenderObject(this.onLayout);

  final VoidCallback onLayout;

  @override
  void performLayout() {
    onLayout();
    super.performLayout();
  }
}
