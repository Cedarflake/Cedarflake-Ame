import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_preview_feedback.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "narrow update guidance remains readable on keyboard focus with one Retry action",
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.windows;
      final semantics = tester.ensureSemantics();
      try {
        var retries = 0;
        await tester.pumpWidget(
          _host(
            size: const Size(48, 240),
            child: LibraryPreviewFeedback.failed(
              locationId: "narrow",
              updateRequired: true,
              onRetry: () => retries++,
            ),
          ),
        );
        expect(find.text(LibraryStrings.previewUpdateRequired), findsNothing);
        final label =
            "${LibraryStrings.previewUpdateRequired}\n${LibraryStrings.retryPreview}";
        expect(find.bySemanticsLabel(label), findsOneWidget);
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pumpAndSettle();
        expect(find.text(label), findsOneWidget);
        expect(tester.getSize(find.text(label)).width, greaterThan(48));
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.pumpAndSettle();
        expect(retries, 1);
        expect(tester.takeException(), isNull);
      } finally {
        semantics.dispose();
        debugDefaultTargetPlatformOverride = null;
      }
    },
  );

  testWidgets(
    "feedback adapts to resize and text scale without changing its allotted size",
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        for (final sample in [
          (size: const Size(48, 240), scale: 1.0, showText: false),
          (size: const Size(320, 240), scale: 2.0, showText: true),
          (size: const Size(320, 48), scale: 2.0, showText: false),
        ]) {
          await tester.pumpWidget(
            _host(
              size: sample.size,
              scale: sample.scale,
              child: const LibraryPreviewFeedback.retrying(
                locationId: "resizing",
              ),
            ),
          );
          expect(
            find.text(LibraryStrings.retryingPreview),
            sample.showText ? findsOneWidget : findsNothing,
          );
          expect(
            find.bySemanticsLabel(LibraryStrings.retryingPreview),
            findsOneWidget,
          );
          expect(
            tester.getSize(find.byType(LibraryPreviewFeedback)),
            sample.size,
          );
          expect(tester.takeException(), isNull);
        }
      } finally {
        semantics.dispose();
      }
    },
  );

  testWidgets(
    "failure guidance fits varied tile constraints with enlarged text",
    (tester) async {
      for (final scale in [1.0, 1.5, 2.0, 3.0]) {
        for (final size in [
          const Size(48, 240),
          const Size(100, 90),
          const Size(160, 120),
          const Size(240, 160),
          const Size(320, 240),
        ]) {
          await tester.pumpWidget(
            _host(
              size: size,
              scale: scale,
              child: LibraryPreviewFeedback.failed(
                locationId: "varied",
                updateRequired: true,
                onRetry: () {},
              ),
            ),
          );
          expect(tester.takeException(), isNull, reason: "$size at $scale");
          final button = find.byKey(const ValueKey("preview-retry-varied"));
          final buttonRect = tester.getRect(button);
          final panelRect = tester.getRect(find.byType(LibraryPreviewFeedback));
          expect(buttonRect.shortestSide, greaterThanOrEqualTo(48));
          expect(panelRect.contains(buttonRect.topLeft), isTrue);
          expect(panelRect.right, greaterThanOrEqualTo(buttonRect.right));
          expect(panelRect.bottom, greaterThanOrEqualTo(buttonRect.bottom));
        }
      }
    },
  );
}

Widget _host({required Size size, required Widget child, double scale = 1}) =>
    MaterialApp(
      theme: buildAmeTheme(),
      builder: (context, body) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(scale)),
        child: body!,
      ),
      home: Scaffold(
        body: Center(
          child: SizedBox.fromSize(size: size, child: child),
        ),
      ),
    );
