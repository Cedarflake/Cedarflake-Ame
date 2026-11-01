import "package:cedarflake_ame/features/library/application/library_classification_annotations.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_label.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";

void main() {
  testWidgets(
    "card status keeps album conflict, hover detail and image opening",
    (tester) async {
      var opened = 0;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryClassificationAnnotationsProvider.overrideWith(
              (ref) async => {
                "one": LibraryClassificationReviewReason.conflict,
              },
            ),
            libraryConflictAnnotationsProvider.overrideWith(
              (ref) async => {"one"},
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: Align(
                alignment: Alignment.topLeft,
                child: LibraryPhotoTile(
                  asset: reviewAsset("one"),
                  width: 180,
                  height: 220,
                  isSelecting: true,
                  isSelected: true,
                  onOpen: (_) => opened++,
                  onToggleSelection: (_) {},
                  onViewInformation: (_) {},
                  onCopyPath: (_) {},
                  onRevealFile: (_) {},
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final album = find.byKey(const ValueKey("album-conflict-one"));
      final classification = find.byKey(
        const ValueKey("classification-review-one"),
      );
      expect(
        tester.getRect(album).bottom,
        lessThan(tester.getRect(classification).top),
      );
      expect(tester.getRect(classification).bottom, lessThanOrEqualTo(220));
      expect(find.byKey(const ValueKey("select-one")), findsOneWidget);
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: const Offset(700, 400));
      addTearDown(mouse.removePointer);
      await mouse.moveTo(tester.getCenter(classification));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 1));
      expect(
        find.text(LibraryClassificationReviewReason.conflict.description),
        findsOneWidget,
      );
      await tester.tap(classification);
      expect(opened, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "routine suggestions stay quiet and failures keep a distinct label",
    (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryClassificationAnnotationsProvider.overrideWith(
              (ref) async => {
                "routine": LibraryClassificationReviewReason.sampling,
                "failed": LibraryClassificationReviewReason.analysisFailed,
              },
            ),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: Column(
                children: [
                  LibraryClassificationLabel(locationId: "routine"),
                  LibraryClassificationLabel(locationId: "missing"),
                  LibraryClassificationLabel(locationId: "failed"),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text("待确认"), findsNothing);
      expect(find.text("分析失败"), findsOneWidget);
      expect(
        find.byTooltip(
          LibraryClassificationReviewReason.analysisFailed.description,
        ),
        findsOneWidget,
      );
    },
  );
}
