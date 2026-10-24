import "dart:math" as math;
import "dart:ui" as ui;

import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/app/window/ame_window_fullscreen.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_asset_information_sheet.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_toolbar.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "actual size survives filmstrip and fullscreen while Escape restores the viewer",
    (tester) async {
      tester.view.physicalSize = const Size(1000, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final asset = await _assetWithImage(tester, width: 600, height: 1000);
      var nativeFullscreen = false;
      var back = 0;
      var organizationActions = 0;
      final fullscreen = AmeWindowFullscreen(
        capturePlacement: () async {},
        setFullscreen: (value) async => nativeFullscreen = value,
        readFullscreen: () async => nativeFullscreen,
      );
      addTearDown(fullscreen.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            ameWindowFullscreenProvider.overrideWithValue(fullscreen),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryImageViewer(
                asset: asset,
                filmstrip: const SizedBox.expand(key: Key("test-filmstrip")),
                organizationActions: IconButton(
                  key: const Key("test-organization-action"),
                  tooltip: "加入收藏夹",
                  onPressed: () => organizationActions++,
                  icon: const Icon(Icons.favorite_outline),
                ),
                onBack: () => back++,
                onInformation: () {},
                onCopyPath: () {},
                onRevealFile: () {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("test-filmstrip")), findsNothing);
      final organizationAction = find.byKey(
        const Key("test-organization-action"),
      );
      final organizationElement = tester.element(organizationAction);
      final actionRect = tester.getRect(organizationAction);
      final minimizeRect = tester.getRect(
        find.byKey(const Key("window-minimize")),
      );
      final dragRect = tester.getRect(
        find.byKey(const Key("viewer-window-drag-region")),
      );
      expect(actionRect.center.dy, minimizeRect.center.dy);
      expect(actionRect.right, lessThan(minimizeRect.left));
      expect(actionRect.left, greaterThanOrEqualTo(dragRect.right));
      await tester.tap(organizationAction);
      expect(organizationActions, 1);
      await tester.tap(find.byKey(const Key("viewer-zoom-mode")));
      await tester.pumpAndSettle();
      expect(find.text("100%"), findsOneWidget);
      await tester.tap(find.byKey(const Key("viewer-filmstrip-toggle")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("test-filmstrip")), findsOneWidget);
      expect(find.text("100%"), findsOneWidget);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(LibraryImageViewer)),
      );
      expect(
        container.read(amePreferencesControllerProvider).viewerOpenBehavior,
        ImageViewerOpenBehavior.automatic,
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.f11);
      await tester.pumpAndSettle();
      expect(nativeFullscreen, isTrue);
      expect(find.byKey(const Key("viewer-window-bar")), findsNothing);
      expect(find.text("100%"), findsOneWidget);
      expect(tester.element(organizationAction), same(organizationElement));
      expect(tester.getRect(organizationAction).top, lessThan(64));
      expect(tester.getRect(organizationAction).right, closeTo(980, .1));
      await tester.tap(organizationAction);
      expect(organizationActions, 2);
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(nativeFullscreen, isFalse);
      expect(back, 0);
      expect(find.byKey(const Key("viewer-window-bar")), findsOneWidget);
      expect(find.text("100%"), findsOneWidget);
      expect(tester.element(organizationAction), same(organizationElement));
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      expect(back, 1);
    },
  );

  testWidgets("viewer actions follow the active asset after navigation", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final assets = [
      _asset(
        sourcePath: r"\\?\G:\CloudLibrary\图片\第一张.jpg",
        displayPath: r"G:\CloudLibrary\图片\第一张.jpg",
      ),
      _asset(
        sourcePath: r"\\?\G:\CloudLibrary\图片\第二张.jpg",
        displayPath: r"G:\CloudLibrary\图片\第二张.jpg",
        locationId: "location-2",
      ),
    ];
    final revealedFiles = <String>[];
    var activeIndex = 0;

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              final activeAsset = assets[activeIndex];
              return Scaffold(
                body: LibraryImageViewer(
                  asset: activeAsset,
                  onBack: () {},
                  onInformation: () {},
                  onCopyPath: () {},
                  onRevealFile: () {
                    revealedFiles.add(activeAsset.sourcePath);
                  },
                  onNext: activeIndex < assets.length - 1
                      ? () => setState(() => activeIndex += 1)
                      : null,
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();

    await _revealViewerFile(tester);
    expect(revealedFiles, [assets.first.sourcePath]);

    await tester.tap(find.byKey(const Key("viewer-next")));
    await tester.pump();
    await _revealViewerFile(tester);
    expect(revealedFiles, [assets.first.sourcePath, assets.last.sourcePath]);
  });

  testWidgets("viewer distinguishes fit-to-window from actual pixel size", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final asset = await _assetWithImage(tester, width: 4000, height: 2000);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: LibraryImageViewer(
              asset: asset,
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

    _expectViewerPixels(tester, asset);
    expect(find.text("4 / 12 张"), findsOneWidget);
    expect(find.text("sample.png"), findsOneWidget);
    expect(find.text("C:\\Pictures\\sample.png"), findsNothing);
    expect(
      find.ancestor(
        of: find.byKey(const Key("viewer-source-path")),
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is Semantics &&
              widget.properties.tooltip == "C:\\Pictures\\sample.png",
        ),
      ),
      findsOneWidget,
    );
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
    final popupSurface = find
        .ancestor(of: copyPathItem, matching: find.byType(Material))
        .first;
    expect(
      tester.getRect(popupSurface).bottom,
      lessThanOrEqualTo(
        tester.getRect(find.byKey(const Key("viewer-more-menu"))).top - 4,
      ),
    );
    final moreButtonCenter = tester.getCenter(
      find.byKey(const Key("viewer-more-menu")),
    );
    await tester.tapAt(moreButtonCenter);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.text("复制路径"), findsNothing);

    expect(find.text("25%"), findsOneWidget);

    final originalPixels = tester.widget<RawImage>(find.byType(RawImage)).image;
    expect(
      tester.widget<RawImage>(find.byType(RawImage)).filterQuality,
      FilterQuality.medium,
    );

    await tester.tap(find.byKey(const Key("viewer-zoom-mode")));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text("100%"), findsOneWidget);

    expect(
      tester.widget<RawImage>(find.byType(RawImage)).filterQuality,
      FilterQuality.low,
    );
    await tester.tap(find.byKey(const Key("viewer-zoom-in")));
    await tester.pumpAndSettle();
    expect(
      tester.widget<RawImage>(find.byType(RawImage)).filterQuality,
      FilterQuality.high,
    );
    expect(
      tester.widget<RawImage>(find.byType(RawImage)).image,
      same(originalPixels),
    );
    await tester.tap(find.byKey(const Key("viewer-zoom-out")));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key("viewer-zoom-mode")));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text("25%"), findsOneWidget);

    final viewportRect = tester.getRect(
      find.byKey(const Key("library-image-interactive-viewer")),
    );
    final controlsRect = tester.getRect(
      find.byKey(const Key("viewer-zoom-controls")),
    );
    expect(viewportRect.bottom, lessThanOrEqualTo(controlsRect.top));
    expect(controlsRect.left, 0);
    expect(controlsRect.right, 1000);

    await tester.tapAt(viewportRect.center);
    await tester.pump(kDoubleTapMinTime);
    await tester.tapAt(viewportRect.center);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text("100%"), findsOneWidget);
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
    expect(find.text("100%"), findsOneWidget);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit1);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(previousCount, 1);
    expect(nextCount, 1);
    expect(find.text("100%"), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.digit0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();
    expect(find.text("500%"), findsOneWidget);
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
    final asset = await _assetWithImage(tester, width: 200, height: 100);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryImageViewer(
              asset: asset,
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

    _expectViewerPixels(tester, asset);
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

    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 250)),
    );
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
    final asset = await _assetWithImage(tester, width: 1000, height: 500);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryImageViewer(
              asset: asset,
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

    _expectViewerPixels(tester, asset);
    final viewer = find.byKey(const Key("library-image-interactive-viewer"));
    await tester.drag(viewer, const Offset(800, 500));
    await tester.pump();

    final interactiveViewer = tester.widget<InteractiveViewer>(viewer);
    final translation = interactiveViewer.transformationController!.value
        .getTranslation();
    expect(translation.x, closeTo(0, 0.01));
    expect(translation.y, closeTo(0, 0.01));

    final zoomIn = find.byKey(const Key("viewer-zoom-in"));
    for (var index = 0; index < 4; index++) {
      await tester.tap(zoomIn);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
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

  testWidgets("viewer groups browsing left and zoom with fullscreen right", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: LibraryViewerToolbar(
              asset: _asset(),
              isFilmstripVisible: false,
              onToggleFilmstrip: () {},
              onInformation: () {},
              onCopyPath: () {},
              onRevealFile: () {},
              positionLabel: "4 / 10000 张",
              isActualSize: true,
              onToggleZoomMode: () {},
              isFullscreen: false,
              onToggleFullscreen: () {},
              sliderValue: 0.5,
              zoomPercent: 100,
              onSliderChanged: (_) {},
              sliderSemanticFormatter: (_) => "100%",
              onZoomOut: () {},
              onZoomIn: () {},
            ),
          ),
        ),
      ),
    );

    final controls = tester.getRect(
      find.byKey(const Key("viewer-zoom-controls")),
    );
    final zoomOut = tester.getRect(find.byKey(const Key("viewer-zoom-out")));
    final zoomIn = tester.getRect(find.byKey(const Key("viewer-zoom-in")));
    final strip = tester.getRect(
      find.byKey(const Key("viewer-filmstrip-toggle")),
    );
    final mode = tester.getRect(find.byKey(const Key("viewer-zoom-mode")));
    final fullscreen = tester.getRect(
      find.byKey(const Key("viewer-fullscreen")),
    );
    expect(strip.center.dx, lessThan(controls.center.dx));
    expect(mode.center.dx, greaterThan(controls.center.dx));
    expect(zoomOut.center.dx, greaterThan(controls.center.dx));
    expect(zoomIn.center.dx, greaterThan(controls.center.dx));
    expect(fullscreen.left, greaterThan(zoomIn.right));
    final summary = find.byKey(const Key("viewer-image-summary"));
    final position = find.byKey(const Key("viewer-position"));
    final information = find.byKey(const Key("viewer-information"));
    final more = find.byKey(const Key("viewer-more-menu"));
    expect(
      tester.getRect(summary).right,
      lessThan(tester.getRect(position).left),
    );
    expect(strip.right, lessThanOrEqualTo(tester.getRect(information).left));
    expect(
      tester.getRect(information).right,
      lessThanOrEqualTo(tester.getRect(more).left),
    );
    expect(tester.getRect(more).right, lessThan(tester.getRect(summary).left));
    expect(tester.getRect(position).right, lessThan(mode.left));
    expect(tester.getRect(more).center.dy, controls.center.dy);
    expect(
      tester.widget<Text>(summary).style,
      tester
          .widget<Text>(find.byKey(const Key("viewer-zoom-percentage")))
          .style,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets("viewer animates programmatic zoom and settles at the target", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final asset = await _assetWithImage(tester, width: 1000, height: 500);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryImageViewer(
              asset: asset,
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

    _expectViewerPixels(tester, asset);
    final viewer = find.byKey(const Key("library-image-interactive-viewer"));
    final interactiveViewer = tester.widget<InteractiveViewer>(viewer);
    final startScale = interactiveViewer.transformationController!.value
        .getMaxScaleOnAxis();

    await tester.tap(find.byKey(const Key("viewer-zoom-in")));
    await tester.pump();
    final initialScale = interactiveViewer.transformationController!.value
        .getMaxScaleOnAxis();
    await tester.pump(const Duration(milliseconds: 90));
    final middleScale = interactiveViewer.transformationController!.value
        .getMaxScaleOnAxis();
    await tester.pump(const Duration(milliseconds: 100));
    final endScale = interactiveViewer.transformationController!.value
        .getMaxScaleOnAxis();

    expect(initialScale, closeTo(startScale, 0.001));
    expect(middleScale, greaterThan(startScale));
    expect(middleScale, lessThan(startScale * 1.25));
    expect(endScale, closeTo(startScale * 1.25, 0.001));
  });

  testWidgets("viewer animates mouse-wheel zoom and settles at the target", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final asset = await _assetWithImage(tester, width: 1000, height: 500);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryImageViewer(
              asset: asset,
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

    _expectViewerPixels(tester, asset);
    final viewer = find.byKey(const Key("library-image-interactive-viewer"));
    final interactiveViewer = tester.widget<InteractiveViewer>(viewer);
    final controller = interactiveViewer.transformationController!;
    final startScale = controller.value.getMaxScaleOnAxis();
    final expectedScale = startScale * math.exp(24 / 180);

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(viewer),
        scrollDelta: const Offset(0, -24),
        kind: PointerDeviceKind.mouse,
      ),
    );
    await tester.pump();
    final initialScale = controller.value.getMaxScaleOnAxis();
    await tester.pump(const Duration(milliseconds: 90));
    final middleScale = controller.value.getMaxScaleOnAxis();
    await tester.pump(const Duration(milliseconds: 100));
    final endScale = controller.value.getMaxScaleOnAxis();

    expect(initialScale, closeTo(startScale, 0.001));
    expect(middleScale, greaterThan(startScale));
    expect(middleScale, lessThan(expectedScale));
    expect(endScale, closeTo(expectedScale, 0.001));
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

Future<void> _revealViewerFile(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("viewer-more-menu")));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
  await tester.tap(find.text(LibraryStrings.openInExplorer));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
}

Future<LibraryAsset> _assetWithImage(
  WidgetTester tester, {
  required int width,
  required int height,
}) async {
  final asset = _asset(width: width, height: height);
  final source = LibrarySourceImage(asset);
  await tester.runAsync(() async {
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    final bounds = Rect.fromLTWH(0, 0, width.toDouble(), height.toDouble());
    canvas.drawRect(bounds, Paint()..color = Colors.indigo);
    canvas.drawRect(
      Rect.fromLTWH(0, 0, width / 3, height / 2),
      Paint()..color = Colors.amber,
    );
    final picture = recorder.endRecording();
    try {
      final image = await picture.toImage(width, height);
      PaintingBinding.instance.imageCache.putIfAbsent(
        source,
        () =>
            OneFrameImageStreamCompleter(Future.value(ImageInfo(image: image))),
      );
    } finally {
      picture.dispose();
    }
  });
  addTearDown(() => source.evict());
  return asset;
}

void _expectViewerPixels(WidgetTester tester, LibraryAsset asset) {
  final image = tester.widget<RawImage>(find.byType(RawImage)).image;
  expect(image?.width, asset.width);
  expect(image?.height, asset.height);
  expect(find.byKey(const Key("viewer-image-loading")), findsNothing);
  expect(find.text("无法打开原图"), findsNothing);
}

LibraryAsset _asset({
  String locationId = "location-1",
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
    locationId: locationId,
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
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
