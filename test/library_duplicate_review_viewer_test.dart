import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review_window.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  testWidgets(
    "viewer navigation crosses a catalog batch without loading all members",
    (tester) async {
      final starts = <int>[];
      final window = LibraryDuplicateReviewWindow<LibraryAsset, int>((
        after,
      ) async {
        final start = after ?? 0;
        starts.add(start);
        return LibraryDuplicateReviewSlice(
          items: List.generate(
            64,
            (index) => reviewAsset("item-${start + index}"),
          ),
          total: 256,
          next: start < 192 ? start + 64 : null,
        );
      });
      addTearDown(window.dispose);
      await window.ensureRange(63, 64);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryDuplicateReviewViewer(
                initialAsset: window.itemAt(63)!,
                initialIndex: 63,
                totalCount: 256,
                loadAsset: (index) async {
                  await window.retryRange(index, index + 1);
                  return window.itemAt(index);
                },
                onBack: () {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();
      expect(starts, [0, 64]);
      expect(
        tester
            .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
            .asset
            .locationId,
        "item-64",
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.pumpAndSettle();
      expect(starts, [0, 64]);
      expect(
        tester
            .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
            .asset
            .locationId,
        "item-63",
      );
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("a late navigation result cannot reopen a disposed viewer", (
    tester,
  ) async {
    final next = Completer<LibraryAsset?>();
    final library = ReviewFixtureLibrary();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [libraryControllerProvider.overrideWith(() => library)],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryDuplicateReviewViewer(
              initialAsset: reviewAsset("one"),
              initialIndex: 0,
              totalCount: 2,
              loadAsset: (_) => next.future,
              onBack: () {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    await tester.pumpWidget(const SizedBox.shrink());
    next.complete(reviewAsset("two"));
    await tester.pumpAndSettle();
    expect(find.byType(LibraryImageViewer), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
