import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header_actions.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final reducedMotion in [false, true]) {
    testWidgets(
      "header row entry, reversal and exit respect reduced motion $reducedMotion",
      (tester) async {
        final reviewing = ValueNotifier(false);
        addTearDown(reviewing.dispose);
        var presses = 0;
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: MediaQuery(
                data: MediaQueryData(disableAnimations: reducedMotion),
                child: ValueListenableBuilder(
                  valueListenable: reviewing,
                  builder: (_, active, _) => Column(
                    children: [
                      LibraryGalleryHeaderActions(
                        primary: const SizedBox(height: 40, width: 180),
                        secondary: active
                            ? SizedBox(
                                key: const Key("second-row"),
                                height: 40,
                                child: TextButton(
                                  onPressed: () => presses++,
                                  child: const Text("退出整理"),
                                ),
                              )
                            : null,
                      ),
                      const Expanded(
                        child: SizedBox.expand(key: Key("gallery")),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
        double galleryTop() =>
            tester.getTopLeft(find.byKey(const Key("gallery"))).dy;
        expect(galleryTop(), 40);
        reviewing.value = true;
        await tester.pump();
        if (reducedMotion) {
          expect(galleryTop(), 84);
        } else {
          expect(galleryTop(), 40);
          await tester.pump(const Duration(milliseconds: 90));
          expect(galleryTop(), inExclusiveRange(40, 84));
        }
        await tester.pumpAndSettle();
        expect(galleryTop(), 84);
        await tester.tap(find.text("退出整理"));
        expect(presses, 1);
        reviewing.value = false;
        await tester.pump();
        if (!reducedMotion) {
          expect(galleryTop(), 84);
          await tester.pump(const Duration(milliseconds: 60));
          final reversingTop = galleryTop();
          expect(reversingTop, inExclusiveRange(40, 84));
          final fade = tester.widget<FadeTransition>(
            find.descendant(
              of: find.byType(LibraryGalleryHeaderActions),
              matching: find.byType(FadeTransition),
            ),
          );
          expect(fade.opacity.value, inExclusiveRange(0, 1));
          reviewing.value = true;
          await tester.pump();
          expect(galleryTop(), reversingTop);
          await tester.pumpAndSettle();
          expect(galleryTop(), 84);
          reviewing.value = false;
          await tester.pump();
        }
        await tester.pumpAndSettle();
        expect(galleryTop(), 40);
        expect(find.byKey(const Key("second-row")), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }
}
