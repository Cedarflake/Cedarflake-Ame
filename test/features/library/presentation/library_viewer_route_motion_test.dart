import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_fade.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final reducedMotion in [false, true]) {
    testWidgets("viewer route shares fade timing (reduced=$reducedMotion)", (
      tester,
    ) async {
      final navigator = GlobalKey<NavigatorState>();
      await tester.pumpWidget(
        MaterialApp(
          navigatorKey: navigator,
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(disableAnimations: reducedMotion),
            child: child!,
          ),
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => Navigator.of(context).push<void>(
                  LibraryViewerRoute(
                    context: context,
                    builder: (_) => const Scaffold(body: Text("viewer")),
                  ),
                ),
                child: const Text("open"),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text("open"));
      await tester.pump();
      await tester.pump();
      final motion = find.byType(LibraryViewerFade);
      final fade = find.descendant(
        of: motion,
        matching: find.byType(FadeTransition),
      );
      double opacity() => tester.widget<FadeTransition>(fade).opacity.value;
      final initialBounds = tester.getRect(find.text("viewer"));
      if (reducedMotion) {
        expect(opacity(), 1);
      } else {
        expect(opacity(), 0);
        await tester.pump(const Duration(milliseconds: 60));
        expect(opacity(), greaterThan(1 / 3));
        expect(opacity(), lessThan(1));
        expect(tester.getRect(find.text("viewer")), initialBounds);
        await tester.pump(const Duration(milliseconds: 120));
        expect(opacity(), 1);
      }
      navigator.currentState!.pop();
      await tester.pump();
      if (!reducedMotion) {
        await tester.pump(const Duration(milliseconds: 40));
        expect(opacity(), greaterThan(0));
        expect(opacity(), lessThan(1));
        expect(tester.getRect(find.text("viewer")), initialBounds);
        await tester.pump(const Duration(milliseconds: 60));
        expect(opacity(), closeTo(0, 0.001));
      }
      await tester.pump(const Duration(milliseconds: 16));
      expect(find.text("viewer"), findsNothing);
      expect(find.text("open"), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }
}
