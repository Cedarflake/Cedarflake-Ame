import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_grid.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  for (final singleColumn in [false, true]) {
    for (final reducedMotion in [false, true]) {
      testWidgets(
        "review singleColumn=$singleColumn shares wheel motion with reducedMotion=$reducedMotion",
        (tester) async {
          await tester.binding.setSurfaceSize(const Size(1280, 900));
          addTearDown(() => tester.binding.setSurfaceSize(null));
          await tester.pumpWidget(
            ProviderScope(
              overrides: [
                libraryControllerProvider.overrideWith(
                  ReviewFixtureLibrary.new,
                ),
              ],
              child: MaterialApp(
                home: MediaQuery(
                  data: MediaQueryData(disableAnimations: reducedMotion),
                  child: Scaffold(
                    body: LibraryDuplicateReviewGrid(
                      itemCount: 100,
                      singleColumn: singleColumn,
                      assetAt: (index) => reviewAsset("item-$index"),
                      itemBuilder: (_, index) => Text("Image $index"),
                    ),
                  ),
                ),
              ),
            ),
          );
          await tester.pumpAndSettle();
          final grid = find.byType(GridView);
          final controller = tester.widget<GridView>(grid).controller!;
          await tester.sendEventToBinding(
            PointerScrollEvent(
              kind: PointerDeviceKind.mouse,
              position: tester.getCenter(grid),
              scrollDelta: const Offset(0, 120),
            ),
          );
          final immediateOffset = controller.offset;
          expect(immediateOffset, greaterThan(0));
          if (reducedMotion) {
            expect(immediateOffset, 120);
            expect(controller.position.isScrollingNotifier.value, isFalse);
          } else {
            expect(immediateOffset, lessThan(120));
            expect(controller.position.isScrollingNotifier.value, isTrue);
            await tester.pump();
            await tester.pump(const Duration(milliseconds: 45));
            expect(controller.offset, greaterThan(immediateOffset));
            expect(controller.offset, lessThan(120));
          }
          await tester.pumpAndSettle();
          expect(controller.offset, closeTo(120, 0.001));
          expect(controller.position.isScrollingNotifier.value, isFalse);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}
