import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("preserves gallery position after closing the image viewer", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(_libraryState()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final photoWall = find.byKey(const Key("library-photo-wall"));
    await tester.drag(photoWall, const Offset(0, -1200));
    await tester.pumpAndSettle();

    final scrollable = find.descendant(
      of: photoWall,
      matching: find.byType(Scrollable),
    );
    final positionBefore = tester.state<ScrollableState>(scrollable).position;
    final offsetBefore = positionBefore.pixels;
    expect(offsetBefore, greaterThan(0));

    final tile = find.byType(LibraryPhotoTile).hitTestable().first;
    final tileRect = tester.getRect(tile);
    await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
    await tester.pump();
    final backButton = find.byKey(const Key("viewer-back-button"));
    expect(backButton, findsOneWidget);
    await tester.tap(backButton);
    await tester.pump();

    final restoredScrollable = find.descendant(
      of: find.byKey(const Key("library-photo-wall")),
      matching: find.byType(Scrollable),
    );
    final positionAfter = tester
        .state<ScrollableState>(restoredScrollable)
        .position;
    expect(identical(positionAfter, positionBefore), isTrue);
    expect(positionAfter.pixels, closeTo(offsetBefore, 0.01));
  });
}

LibraryState _libraryState() {
  final assets = [
    for (var index = 0; index < 120; index++)
      LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: "root-1",
        sourcePath: "C:\\Pictures\\$index.jpg",
        displayPath: "C:\\Pictures\\$index.jpg",
        relativePath: "$index.jpg",
        previewPath: "C:\\Ame\\previews\\$index.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: index,
        width: 4,
        height: 3,
        previewStatus: LibraryPreviewStatus.ready,
      ),
  ];
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      catalogPath: "C:\\Ame\\catalog.db",
      revision: BigInt.one,
      queryId: "viewer-position",
      roots: [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          createdUnixMs: 1,
          assetCount: assets.length,
          issueCount: 0,
        ),
      ],
      assets: assets,
    ),
  );
}
