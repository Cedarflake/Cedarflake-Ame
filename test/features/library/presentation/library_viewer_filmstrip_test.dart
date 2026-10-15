import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "large filmstrip builds visible thumbnails, centers selection and selects neighbors",
    (tester) async {
      final assets = List.generate(
        10000,
        (index) => LibraryAsset(
          assetId: "asset-$index",
          locationId: "location-$index",
          rootId: "root",
          activeScanId: "scan",
          sourcePath: "unused/photo-$index.jpg",
          displayPath: "photo-$index.jpg",
          relativePath: "photo-$index.jpg",
          previewPath: "",
          fileSize: BigInt.from(2000 + index),
          modifiedUnixMs: index,
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          width: 640,
          height: 480,
          previewStatus: LibraryPreviewStatus.failed,
        ),
      );
      LibraryAsset? selected;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState(assets: assets),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  height: 104,
                  width: 700,
                  child: LibraryViewerFilmstrip(
                    selection: assets[5000],
                    onSelected: (asset) => selected = asset,
                    loadPrevious: () async {},
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(InkWell).evaluate().length, lessThanOrEqualTo(11));
      final current = find.byKey(const ValueKey("location-5000"));
      expect(current.hitTestable(), findsOneWidget);
      final strip = find.byKey(const Key("viewer-filmstrip"));
      expect(
        tester.getCenter(current).dx,
        closeTo(tester.getCenter(strip).dx, 0.1),
      );
      await tester.tap(find.byKey(const ValueKey("location-5001")));
      expect(selected?.locationId, "location-5001");
      expect(find.byKey(const ValueKey("location-0")), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}
