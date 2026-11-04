import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";

import "../../../support/application_test_bindings.dart";

void main() {
  testWidgets(
    "card preserves album conflicts without reading analysis failures",
    (tester) async {
      var opened = 0;
      await tester.pumpWidget(
        ProviderScope(
          overrides: applicationTestOverrides([
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryClassificationReviewServiceProvider.overrideWith(
              (ref) => throw StateError("Card must not read analysis status"),
            ),
            libraryConflictAnnotationsProvider.overrideWith(
              (ref) async => {"one"},
            ),
          ]),
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
      expect(tester.getRect(album).bottom, lessThanOrEqualTo(220));
      expect(find.text("分析失败"), findsNothing);
      expect(find.byKey(const ValueKey("select-one")), findsOneWidget);
      expect(find.text("相册归属冲突"), findsOneWidget);
      await tester.tapAt(tester.getCenter(album));
      expect(opened, 1);
      expect(tester.takeException(), isNull);
    },
  );
}
