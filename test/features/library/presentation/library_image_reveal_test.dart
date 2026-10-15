import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/presentation/widgets/library_image_reveal.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  late ui.Image pixels;

  setUp(() async {
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    canvas.drawRect(
      const Rect.fromLTWH(0, 0, 8, 4),
      Paint()..color = Colors.blue,
    );
    canvas.drawRect(
      const Rect.fromLTWH(0, 4, 8, 4),
      Paint()..color = Colors.orange,
    );
    final picture = recorder.endRecording();
    pixels = await picture.toImage(8, 8);
    picture.dispose();
  });

  tearDown(() {
    PaintingBinding.instance.imageCache.clear();
    PaintingBinding.instance.imageCache.clearLiveImages();
    pixels.dispose();
  });

  testWidgets("first decoded frame fades without changing its rectangle", (
    tester,
  ) async {
    final source = _ControlledImage();
    await tester.pumpWidget(_host(source));
    final bounds = tester.getRect(find.byType(LibraryImageReveal));
    await tester.pump(const Duration(milliseconds: 500));
    expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNull);
    source.show(pixels);
    await tester.pump();
    expect(_opacity(tester), 0);
    await tester.pump(const Duration(milliseconds: 60));
    expect(_opacity(tester), greaterThan(0));
    expect(_opacity(tester), lessThan(1));
    expect(tester.getRect(find.byType(LibraryImageReveal)), bounds);
    await tester.pump(const Duration(milliseconds: 60));
    expect(_opacity(tester), 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "preview remains beneath new pixels until their reveal completes",
    (tester) async {
      final source = _ControlledImage();
      await tester.pumpWidget(
        _host(source, placeholder: const Text("preview")),
      );
      source.show(pixels);
      await tester.pump();
      final bounds = tester.getRect(find.byType(LibraryImageReveal));
      expect(find.text("preview"), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 60));
      expect(find.text("preview"), findsOneWidget);
      expect(tester.getRect(find.byType(LibraryImageReveal)), bounds);
      await tester.pump(const Duration(milliseconds: 60));
      expect(find.text("preview"), findsNothing);
      expect(_opacity(tester), 1);
    },
  );

  for (final mode in ["cached", "reduced motion", "offstage"]) {
    testWidgets("$mode images appear immediately", (tester) async {
      final source = _ControlledImage();
      if (mode == "cached") source.show(pixels);
      await tester.pumpWidget(
        _host(
          source,
          disableAnimations: mode == "reduced motion",
          tickerEnabled: mode != "offstage",
        ),
      );
      if (mode != "cached") {
        source.show(pixels);
        await tester.pump();
      }
      expect(_opacity(tester), 1);
      expect(tester.binding.transientCallbackCount, 0);
    });
  }

  testWidgets("scrolling takes over a running fade immediately", (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final source = _ControlledImage();
    await tester.pumpWidget(_host(source, controller: controller));
    source.show(pixels);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 30));
    expect(_opacity(tester), lessThan(1));
    controller.jumpTo(25);
    expect(_opacity(tester), 1);
    await tester.pump();
    expect(
      tester.getSize(find.byType(LibraryImageReveal)),
      const Size(240, 160),
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    "late frames cannot revive an image after replacement or disposal",
    (tester) async {
      final oldSource = _ControlledImage();
      final currentSource = _ControlledImage();
      await tester.pumpWidget(_host(oldSource));
      await tester.pumpWidget(_host(currentSource, disableAnimations: true));
      oldSource.show(pixels);
      await tester.pump();
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNull);
      currentSource.show(pixels);
      await tester.pump();
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
      expect(_opacity(tester), 1);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump(const Duration(milliseconds: 200));
      expect(tester.takeException(), isNull);
      expect(tester.binding.transientCallbackCount, 0);
    },
  );
}

double _opacity(WidgetTester tester) =>
    tester.widget<Image>(find.byType(Image)).opacity!.value;

Widget _host(
  _ControlledImage source, {
  bool disableAnimations = false,
  bool tickerEnabled = true,
  ScrollController? controller,
  Widget? placeholder,
}) {
  final image = SizedBox(
    width: 240,
    height: 160,
    child: LibraryImageReveal(
      image: source,
      revealIdentity: source,
      placeholder: placeholder,
    ),
  );
  return MaterialApp(
    home: MediaQuery(
      data: MediaQueryData(disableAnimations: disableAnimations),
      child: TickerMode(
        enabled: tickerEnabled,
        child: Center(
          child: controller == null
              ? image
              : SizedBox(
                  width: 240,
                  height: 300,
                  child: ListView(
                    controller: controller,
                    children: [image, const SizedBox(height: 1000)],
                  ),
                ),
        ),
      ),
    ),
  );
}

class _ControlledImage extends ImageProvider<_ControlledImage> {
  final _completer = _Frames();

  void show(ui.Image image) => _completer.show(image.clone());

  @override
  Future<_ControlledImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(
    _ControlledImage key,
    ImageDecoderCallback decode,
  ) => _completer;
}

class _Frames extends ImageStreamCompleter {
  void show(ui.Image image) => setImage(ImageInfo(image: image));
}
