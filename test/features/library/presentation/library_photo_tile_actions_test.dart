import "dart:async";

import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_card_metrics.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final count in [2, 12, 123, 12345, 1000000, 9223372036854775807]) {
    for (final scale in [1.0, 2.0]) {
      testWidgets(
        "card keeps $count copies and selection visible at ${scale}x text",
        (tester) async {
          final selecting = ValueNotifier(false);
          addTearDown(selecting.dispose);
          final groups = Completer<Map<String, LibraryDuplicateGroup>>();
          var selections = 0;
          await tester.pumpWidget(
            ProviderScope(
              overrides: [
                libraryGalleryDuplicateAnnotationsProvider.overrideWith(
                  (_) => groups.future,
                ),
              ],
              child: MaterialApp(
                theme: ThemeData(useMaterial3: true),
                builder: (context, child) => MediaQuery(
                  data: MediaQuery.of(
                    context,
                  ).copyWith(textScaler: TextScaler.linear(scale)),
                  child: child!,
                ),
                home: Scaffold(
                  body: Align(
                    alignment: Alignment.topLeft,
                    child: Builder(
                      builder: (context) => ValueListenableBuilder(
                        valueListenable: selecting,
                        builder: (_, isSelecting, _) => LibraryPhotoTile(
                          asset: _portrait(),
                          width: LibraryGalleryCardMetrics.minimumWidthOf(
                            context,
                            maximumLocationCount: count,
                          ),
                          height: 96,
                          isSelecting: isSelecting,
                          isSelected: false,
                          onOpen: (_) {},
                          onToggleSelection: (_) => selections++,
                          onViewInformation: (_) {},
                          onCopyPath: (_) {},
                          onRevealFile: (_) {},
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
          final initialTile = tester.getRect(find.byType(LibraryPhotoTile));
          groups.complete({
            "location": LibraryDuplicateGroup(
              digest: "a" * 64,
              byteLength: 20,
              locationCount: count,
              physicalFileCount: count,
              representativeLocationId: "location",
              representativePath: r"C:\fixture\portrait.jpg",
            ),
          });
          await tester.pump();
          final tile = tester.getRect(find.byType(LibraryPhotoTile));
          expect(tile, initialTile);
          final badge = find.byKey(const ValueKey("duplicate-badge-location"));
          final badgeBeforeSelection = tester.getRect(badge);
          selecting.value = true;
          await tester.pump();
          final badgeRect = tester.getRect(badge);
          final checkbox = find.byKey(const ValueKey("select-location"));
          final checkboxRect = tester.getRect(checkbox);
          expect(badgeRect, badgeBeforeSelection);
          _expectContained(tile, badgeRect);
          _expectContained(tile, checkboxRect);
          expect(badgeRect.right, lessThanOrEqualTo(checkboxRect.left - 4));
          _expectContained(
            tile,
            tester.getRect(
              find.descendant(of: badge, matching: find.byType(Icon)),
            ),
          );
          final paragraph = tester.renderObject<RenderParagraph>(
            find.text("$count"),
          );
          final origin = paragraph.localToGlobal(Offset.zero);
          final boxes = paragraph.getBoxesForSelection(
            TextSelection(baseOffset: 0, extentOffset: "$count".length),
          );
          expect(boxes, hasLength(1));
          for (final box in boxes) {
            _expectContained(tile, box.toRect().shift(origin));
          }
          expect(paragraph.didExceedMaxLines, isFalse);
          await tester.tap(checkbox);
          await tester.pump();
          expect(selections, 1);
          expect(tester.takeException(), isNull);
        },
      );
    }
  }
}

void _expectContained(Rect outer, Rect inner) {
  expect(inner.left, greaterThanOrEqualTo(outer.left));
  expect(inner.top, greaterThanOrEqualTo(outer.top));
  expect(inner.right, lessThanOrEqualTo(outer.right));
  expect(inner.bottom, lessThanOrEqualTo(outer.bottom));
}

LibraryAsset _portrait() => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  sourcePath: r"C:\fixture\portrait.jpg",
  displayPath: r"C:\fixture\portrait.jpg",
  relativePath: "portrait.jpg",
  previewPath: "",
  fileSize: BigInt.from(20),
  modifiedUnixMs: 1,
  width: 20,
  height: 1000,
  previewStatus: LibraryPreviewStatus.pending,
);
