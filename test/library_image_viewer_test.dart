import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_asset_information_sheet.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("viewer distinguishes fit-to-window from actual pixel size", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryImageViewer(
            asset: _asset(width: 200, height: 100),
            position: 4,
            totalItems: 12,
            onBack: () {},
            onInformation: () {},
            onCopyPath: () {},
            onRevealFile: () {},
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text("4 / 12"), findsOneWidget);
    expect(find.text("100%"), findsNothing);

    await tester.tap(find.byKey(const Key("viewer-actual-size")));
    await tester.pump();
    expect(find.text("100%"), findsOneWidget);

    await tester.tap(find.byKey(const Key("viewer-fit")));
    await tester.pump();
    expect(find.text("100%"), findsNothing);
  });

  testWidgets("viewer exposes adjacent navigation through controls and keys", (
    tester,
  ) async {
    var previousCount = 0;
    var nextCount = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryImageViewer(
            asset: _asset(),
            onBack: () {},
            onInformation: () {},
            onCopyPath: () {},
            onRevealFile: () {},
            onPrevious: () => previousCount++,
            onNext: () => nextCount++,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    await tester.tap(find.byKey(const Key("viewer-previous")));
    await tester.tap(find.byKey(const Key("viewer-next")));
    await tester.pump();
    expect(previousCount, 1);
    expect(nextCount, 1);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    expect(previousCount, 2);
    expect(nextCount, 2);
  });

  testWidgets("viewer keeps a fitted image inside the viewport while panning", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryImageViewer(
            asset: _asset(width: 1000, height: 500),
            onBack: () {},
            onInformation: () {},
            onCopyPath: () {},
            onRevealFile: () {},
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    final viewer = find.byKey(const Key("library-image-interactive-viewer"));
    await tester.drag(viewer, const Offset(800, 500));
    await tester.pump();

    final interactiveViewer = tester.widget<InteractiveViewer>(viewer);
    final translation = interactiveViewer.transformationController!.value
        .getTranslation();
    expect(translation.x, closeTo(0, 0.01));
    expect(translation.y, closeTo(0, 0.01));

    final zoomIn = find.byTooltip("放大（Ctrl++）");
    for (var index = 0; index < 4; index++) {
      await tester.tap(zoomIn);
      await tester.pump();
    }
    await tester.drag(viewer, const Offset(1600, 1000));
    await tester.pump();

    final viewportSize = tester.getSize(viewer);
    final zoomedMatrix = interactiveViewer.transformationController!.value;
    final zoomedScale = zoomedMatrix.getMaxScaleOnAxis();
    final zoomedTranslation = zoomedMatrix.getTranslation();
    final fitScale = viewportSize.width / 1000;
    final imageWidth = 1000 * fitScale * zoomedScale;
    final imageHeight = 500 * fitScale * zoomedScale;
    final fittedTop = (viewportSize.height - 500 * fitScale) / 2;
    final imageLeft = zoomedTranslation.x;
    final imageTop = zoomedTranslation.y + fittedTop * zoomedScale;
    expect(imageLeft, lessThanOrEqualTo(0.01));
    expect(imageTop, lessThanOrEqualTo(0.01));
    expect(imageLeft + imageWidth, greaterThanOrEqualTo(viewportSize.width));
    expect(imageTop + imageHeight, greaterThanOrEqualTo(viewportSize.height));
    await tester.pump(const Duration(milliseconds: 100));
  });

  testWidgets("viewer information presents source and filesystem dates", (
    tester,
  ) async {
    final asset = _asset(
      captureTime: const LibraryCaptureTimeEvidence(
        localTime: "2025-07-08T09:10:11",
        source: LibraryCaptureTimeSource.exifDateTimeOriginal,
        rawValue: "2025:07:08 09:10:11",
      ),
      createdUnixMs: DateTime(2025, 7, 9, 10, 11, 12).millisecondsSinceEpoch,
      modifiedUnixMs: DateTime(2025, 7, 10, 11, 12, 13).millisecondsSinceEpoch,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => FilledButton(
            onPressed: () => showLibraryAssetInformation(context, asset),
            child: const Text("打开信息"),
          ),
        ),
      ),
    );

    await tester.tap(find.text("打开信息"));
    await tester.pumpAndSettle();

    expect(find.text("PNG"), findsOneWidget);
    expect(find.text("2025-07-08 09:10:11"), findsOneWidget);
    expect(find.text("相机原始拍摄时间"), findsOneWidget);
    expect(find.text("2025-07-09 10:11:12"), findsOneWidget);
    expect(find.text("2025-07-10 11:12:13"), findsOneWidget);
  });
}

LibraryAsset _asset({
  int width = 4000,
  int height = 2000,
  int? createdUnixMs,
  int modifiedUnixMs = 1,
  LibraryCaptureTimeEvidence? captureTime,
}) {
  return LibraryAsset(
    assetId: "asset-1",
    locationId: "location-1",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\sample.png",
    relativePath: "sample.png",
    previewPath: "",
    fileSize: BigInt.from(2048),
    createdUnixMs: createdUnixMs,
    modifiedUnixMs: modifiedUnixMs,
    width: width,
    height: height,
    previewStatus: LibraryPreviewStatus.failed,
    captureTime: captureTime,
  );
}
