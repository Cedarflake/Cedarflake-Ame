import "package:cedarflake_ame/app/ame_menu.dart";
import "package:cedarflake_ame/app/ame_theme.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_asset_information_sheet.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_controls.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
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
      ProviderScope(
        child: MaterialApp(
          theme: buildAmeTheme(),
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
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text("4 / 12"), findsOneWidget);
    expect(find.text("sample.png"), findsOneWidget);
    expect(find.text("C:\\Pictures\\sample.png"), findsNothing);
    expect(
      tester.getSize(find.byKey(const Key("viewer-source-path"))).width,
      lessThan(200),
    );
    expect(find.byKey(const Key("viewer-window-drag-region")), findsOneWidget);
    expect(find.byKey(const Key("window-minimize")), findsOneWidget);
    expect(find.byKey(const Key("window-maximize")), findsOneWidget);
    expect(find.byKey(const Key("window-close")), findsOneWidget);
    expect(
      tester.getRect(find.byKey(const Key("window-close"))).right,
      closeTo(992, 0.01),
    );
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    await mouse.moveTo(
      tester.getCenter(find.byKey(const Key("viewer-source-path"))),
    );
    await tester.pump(const Duration(milliseconds: 600));
    expect(find.text("C:\\Pictures\\sample.png"), findsOneWidget);

    await tester.tap(find.byKey(const Key("viewer-more-menu")));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    final copyPathItem = find.ancestor(
      of: find.text("复制路径"),
      matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
    );
    final copyPathItemRect = tester.getRect(copyPathItem);
    expect(find.byType(AmeMenuItemContent), findsNWidgets(2));
    expect(copyPathItemRect.width, lessThan(280));
    expect(copyPathItemRect.right, lessThanOrEqualTo(984));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));

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
      ProviderScope(
        child: MaterialApp(
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

  testWidgets("viewer reclaims focus from the offstage gallery", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    var isViewerOpen = false;
    var previousCount = 0;
    var nextCount = 0;
    var backCount = 0;

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) => Scaffold(
              body: IndexedStack(
                index: isViewerOpen ? 1 : 0,
                children: [
                  Center(
                    child: FilledButton(
                      key: const Key("open-viewer-from-gallery"),
                      autofocus: true,
                      onPressed: () => setState(() => isViewerOpen = true),
                      child: const Text("打开预览"),
                    ),
                  ),
                  if (isViewerOpen)
                    LibraryImageViewer(
                      asset: _asset(width: 200, height: 100),
                      onBack: () => backCount++,
                      onInformation: () {},
                      onCopyPath: () {},
                      onRevealFile: () {},
                      onPrevious: () => previousCount++,
                      onNext: () => nextCount++,
                    )
                  else
                    const SizedBox.shrink(),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key("open-viewer-from-gallery")));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(
      FocusManager.instance.primaryFocus?.debugLabel,
      "library-image-viewer",
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit1);
    await tester.pump();
    expect(previousCount, 1);
    expect(nextCount, 1);
    expect(find.text("100%"), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.digit0);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();
    expect(find.text("100%"), findsNothing);
    expect(backCount, 1);
  });

  testWidgets("viewer applies persisted opening and mouse-wheel behavior", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    var previousCount = 0;
    var nextCount = 0;

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryImageViewer(
              asset: _asset(width: 200, height: 100),
              wheelBehavior: ImageViewerWheelBehavior.previousOrNext,
              openBehavior: ImageViewerOpenBehavior.actualSize,
              onBack: () {},
              onInformation: () {},
              onCopyPath: () {},
              onRevealFile: () {},
              onPrevious: () => previousCount++,
              onNext: () => nextCount++,
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text("100%"), findsOneWidget);
    final viewer = find.byKey(const Key("library-image-interactive-viewer"));
    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(viewer),
        scrollDelta: const Offset(0, 24),
      ),
    );
    await tester.pump();
    expect(nextCount, 1);

    await tester.pump(const Duration(milliseconds: 250));
    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(viewer),
        scrollDelta: const Offset(0, -24),
      ),
    );
    await tester.pump();
    expect(previousCount, 1);
  });

  testWidgets("viewer keeps a fitted image inside the viewport while panning", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
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

    final zoomIn = find.byTooltip("放大（+ / Ctrl++）");
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

  testWidgets("viewer zoom controls keep action groups separated", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: LibraryViewerZoomControls(
              sliderValue: 0.5,
              zoomPercent: 100,
              canZoomOut: true,
              canZoomIn: true,
              canShowActualSize: true,
              onSliderChanged: (_) {},
              sliderSemanticFormatter: (_) => "100%",
              onZoomOut: () {},
              onZoomIn: () {},
              onFitToWindow: () {},
              onShowActualSize: () {},
            ),
          ),
        ),
      ),
    );

    final zoomIn = tester.getRect(find.byTooltip("放大（+ / Ctrl++）"));
    final divider = tester.getRect(
      find.byKey(const Key("viewer-zoom-group-divider")),
    );
    final fit = tester.getRect(find.byKey(const Key("viewer-fit")));
    final actualSize = tester.getRect(
      find.byKey(const Key("viewer-actual-size")),
    );

    expect(divider.left - zoomIn.right, greaterThanOrEqualTo(0));
    expect(fit.left - divider.right, greaterThanOrEqualTo(0));
    expect(actualSize.left - fit.right, greaterThanOrEqualTo(4));
  });

  testWidgets("viewer information presents source and filesystem dates", (
    tester,
  ) async {
    final asset = _asset(
      sourcePath: r"\\?\G:\图片\本机照片\2026\08\sample.png",
      displayPath: r"G:\图片\本机照片\2026\08\sample.png",
      relativePath: r"本机照片\2026\08\sample.png",
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

    expect(find.text("sample.png"), findsOneWidget);
    expect(find.text(r"G:\图片\本机照片\2026\08\sample.png"), findsOneWidget);
    expect(find.textContaining(r"\\?\"), findsNothing);
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
  String sourcePath = r"C:\Pictures\sample.png",
  String? displayPath,
  String relativePath = "sample.png",
  int? createdUnixMs,
  int modifiedUnixMs = 1,
  LibraryCaptureTimeEvidence? captureTime,
}) {
  return LibraryAsset(
    assetId: "asset-1",
    locationId: "location-1",
    rootId: "root-1",
    sourcePath: sourcePath,
    displayPath: displayPath ?? sourcePath,
    relativePath: relativePath,
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
