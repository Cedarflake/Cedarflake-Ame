import "dart:async";
import "dart:io";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip_slot.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  late ui.Image firstPixels;
  late ui.Image replacementPixels;

  setUp(() async {
    firstPixels = await _pixels(16, 12);
    replacementPixels = await _pixels(32, 24);
  });

  tearDown(() {
    PaintingBinding.instance.imageCache.clear();
    PaintingBinding.instance.imageCache.clearLiveImages();
    firstPixels.dispose();
    replacementPixels.dispose();
  });

  testWidgets("filmstrip keeps its placeholder until pixels are visible", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final frames = await _cacheFrames(asset);
    await _mount(tester, asset, _PreviewController());
    final bounds = tester.getRect(find.byKey(const ValueKey("location")));
    expect(_rawImage(tester).image, isNull);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);

    frames.show(firstPixels);
    await tester.pump();
    expect(_rawImage(tester).image!.isCloneOf(firstPixels), isTrue);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 60));
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 60));
    expect(find.byIcon(Symbols.image_rounded), findsNothing);
    expect(_opacity(tester), 1);
    expect(tester.getRect(find.byKey(const ValueKey("location"))), bounds);
    expect(tester.takeException(), isNull);
  });

  testWidgets("filmstrip keeps decoded pixels during a preview upgrade", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final first = await _cacheFrames(asset);
    first.show(firstPixels);
    final controller = _PreviewController();
    await _mount(tester, asset, controller);
    expect(_opacity(tester), 1);

    final replacement = _asset("larger.jpg");
    final next = await _cacheFrames(replacement);
    controller.replace(replacement);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(_rawImage(tester).image?.isCloneOf(firstPixels), isTrue);
    expect(find.byIcon(Symbols.image_rounded), findsNothing);
    expect(_opacity(tester), 1);

    next.show(replacementPixels);
    await tester.pump();
    expect(_rawImage(tester).image!.isCloneOf(replacementPixels), isTrue);
    expect(_opacity(tester), 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets("a changed source cannot retain the previous source's pixels", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final first = await _cacheFrames(asset);
    first.show(firstPixels);
    final controller = _PreviewController();
    await _mount(tester, asset, controller);

    final replacement = _asset("rewritten.jpg", generation: 2);
    final next = await _cacheFrames(replacement);
    controller.replace(replacement);
    await tester.pump();
    expect(_rawImage(tester).image, isNull);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    next.show(replacementPixels);
    await tester.pumpAndSettle();
    expect(_rawImage(tester).image!.isCloneOf(replacementPixels), isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "reopening a retained strip preserves pixels without another fade",
    (tester) async {
      final asset = _asset("retained.jpg");
      final frames = await _cacheFrames(asset);
      frames.show(firstPixels);
      final visible = ValueNotifier(true);
      addTearDown(visible.dispose);
      await _mount(tester, asset, _PreviewController(), visibility: visible);
      final originalState = tester.state(find.byType(Image));
      expect(_rawImage(tester).image!.isCloneOf(firstPixels), isTrue);
      visible.value = false;
      await tester.pumpAndSettle();
      PaintingBinding.instance.imageCache.clear();
      visible.value = true;
      await tester.pump();
      expect(tester.state(find.byType(Image)), same(originalState));
      expect(_rawImage(tester).image!.isCloneOf(firstPixels), isTrue);
      expect(_opacity(tester), 1);
      expect(find.byIcon(Symbols.image_rounded), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a missing cached thumbnail repairs once and displays the new result",
    (tester) async {
      final asset = _asset("missing.jpg");
      final frames = await _cacheFrames(asset);
      final controller = _RepairController();
      addTearDown(controller.changes.close);
      await _mount(tester, asset, controller);
      frames.fail();
      await tester.pump();
      await tester.pump();
      expect(controller.repairs, 1);
      expect(find.byIcon(Symbols.broken_image_rounded), findsOneWidget);
      await tester.pump(const Duration(seconds: 1));
      expect(controller.repairs, 1);

      final repaired = _asset("repaired.jpg");
      (await _cacheFrames(repaired)).show(replacementPixels);
      controller.publish(repaired);
      await tester.pumpAndSettle();
      expect(_rawImage(tester).image?.isCloneOf(replacementPixels), isTrue);
      expect(find.byIcon(Symbols.broken_image_rounded), findsNothing);
      expect(controller.repairs, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("a hidden thumbnail cannot begin automatic cache repair", (
    tester,
  ) async {
    final asset = _asset("hidden-missing.jpg");
    final frames = await _cacheFrames(asset);
    final controller = _RepairController();
    final visible = ValueNotifier(true);
    addTearDown(visible.dispose);
    addTearDown(controller.changes.close);
    await _mount(tester, asset, controller, visibility: visible);
    visible.value = false;
    frames.fail();
    await tester.pumpAndSettle();
    expect(controller.repairs, 0);
    visible.value = true;
    await tester.pump();
    await tester.pump();
    expect(controller.repairs, 1);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _mount(
  WidgetTester tester,
  LibraryAsset asset,
  _PreviewController controller, {
  ValueNotifier<bool>? visibility,
}) async {
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryControllerProvider.overrideWith(() => controller),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState(assets: [asset]),
        ),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 400,
              child: visibility == null
                  ? SizedBox(
                      height: 104,
                      child: LibraryViewerFilmstrip(
                        selection: asset,
                        onSelected: (_) {},
                        loadPrevious: () async {},
                      ),
                    )
                  : ValueListenableBuilder(
                      valueListenable: visibility,
                      builder: (context, visible, _) =>
                          LibraryViewerFilmstripSlot(
                            visible: visible,
                            child: LibraryViewerFilmstrip(
                              selection: asset,
                              onSelected: (_) {},
                              loadPrevious: () async {},
                            ),
                          ),
                    ),
            ),
          ),
        ),
      ),
    ),
  );
}

RawImage _rawImage(WidgetTester tester) =>
    tester.widget<RawImage>(find.byType(RawImage));

double _opacity(WidgetTester tester) =>
    tester.widget<Image>(find.byType(Image)).opacity?.value ?? 1;

Future<_Frames> _cacheFrames(LibraryAsset asset) async {
  final image = ResizeImage(FileImage(File(asset.previewPath)), width: 128);
  final key = await image.obtainKey(ImageConfiguration.empty);
  final frames = _Frames();
  PaintingBinding.instance.imageCache.putIfAbsent(key, () => frames);
  return frames;
}

Future<ui.Image> _pixels(int width, int height) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawColor(Colors.blue, BlendMode.src);
  canvas.drawRect(
    Rect.fromLTWH(0, 0, width / 2, height / 2),
    Paint()..color = Colors.orange,
  );
  final picture = recorder.endRecording();
  final pixels = await picture.toImage(width, height);
  picture.dispose();
  return pixels;
}

class _Frames extends ImageStreamCompleter {
  void show(ui.Image image) => setImage(ImageInfo(image: image.clone()));

  void fail() => reportError(
    exception: const FileSystemException("Missing preview"),
    silent: true,
  );
}

class _PreviewController extends LibraryController {
  void replace(LibraryAsset asset) => state = state.copyWith(assets: [asset]);
}

class _RepairController extends _PreviewController {
  final changes = StreamController<void>.broadcast(sync: true);
  LibraryAsset? _published;
  int repairs = 0;

  @override
  Stream<void> watchPreview(String locationId) => changes.stream;

  @override
  LibraryAsset resolvePreview(LibraryAsset asset) => _published ?? asset;

  @override
  void requestPreview(
    LibraryAsset asset, {
    bool retry = false,
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int previewEdge = 512,
  }) {
    expect(retry, isTrue);
    repairs++;
  }

  void publish(LibraryAsset asset) {
    _published = asset;
    changes.add(null);
  }
}

LibraryAsset _asset(String previewPath, {int generation = 1}) => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unused/photo.jpg",
  displayPath: "photo.jpg",
  relativePath: "photo.jpg",
  previewPath: previewPath,
  fileSize: BigInt.from(2000),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.from(generation),
  width: 640,
  height: 480,
  previewStatus: LibraryPreviewStatus.ready,
);
