import "dart:async";
import "dart:convert";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image_stream.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("a first-frame failure after closing the viewer is retired", (
    tester,
  ) async {
    final codec = _Codec();
    final source = _SourceImage(Future.value(codec));
    await tester.pumpWidget(_viewer(source));
    await tester.pump();
    expect(codec.frameRequests, 1);
    source.retire();
    await source.evict();
    await tester.pumpWidget(const SizedBox.shrink());
    await _pumpRetirement(tester);
    expect(codec.disposals, 1);
    codec.frame.completeError(StateError("late native first-frame failure"));
    await tester.pump();
    expect(codec.disposals, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets("an old first-frame failure cannot replace the new selection", (
    tester,
  ) async {
    final oldCodec = _Codec();
    final oldSource = _SourceImage(Future.value(oldCodec));
    final newCodec = _Codec();
    final newSource = _SourceImage(Future.value(newCodec));
    await tester.pumpWidget(_viewer(oldSource));
    await tester.pump();
    expect(oldCodec.frameRequests, 1);
    oldSource.retire();
    await oldSource.evict();
    await tester.pumpWidget(_viewer(newSource));
    await _pumpRetirement(tester);
    expect(oldCodec.disposals, 1);
    expect(newCodec.frameRequests, 1);
    oldCodec.frame.completeError(StateError("obsolete frame failed"));
    await tester.pump();
    expect(find.text("source-frame-error"), findsNothing);
    final frame = await _createFrame(tester);
    newCodec.frame.complete(frame);
    await _waitForImage(tester);
    expect(frame.image.debugDisposed, isTrue);
    expect(oldCodec.disposals, 1);
    expect(newCodec.disposals, 1);
    newSource.retire();
    await newSource.evict();
    await tester.pumpWidget(const SizedBox.shrink());
    await _pumpRetirement(tester);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "a current native first-frame error reaches the visible error builder",
    (tester) async {
      final codec = _Codec();
      final source = _SourceImage(Future.value(codec));
      final errors = <Object>[];
      await tester.pumpWidget(_viewer(source, errors: errors));
      await tester.pump();
      expect(codec.frameRequests, 1);
      final failure = StateError("current native first-frame failure");
      codec.frame.completeError(failure);
      // Frame completion first reaches Image's setState through the wrapped
      // codec future; errorBuilder runs in the following rendering frame.
      for (var attempt = 0; attempt < 10 && errors.isEmpty; attempt += 1) {
        await tester.pump(const Duration(milliseconds: 20));
      }
      expect(errors, isNotEmpty);
      expect(errors.every((error) => identical(error, failure)), isTrue);
      expect(find.text("source-frame-error"), findsOneWidget);
      source.retire();
      await source.evict();
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(codec.disposals, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a successful frame arriving after disposal releases its image exactly once",
    (tester) async {
      final codec = _Codec();
      final source = _SourceImage(Future.value(codec));
      await tester.pumpWidget(_viewer(source));
      await tester.pump();
      expect(codec.frameRequests, 1);
      source.retire();
      await source.evict();
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(codec.disposals, 1);
      final frame = await _createFrame(tester);
      expect(frame.image.debugDisposed, isFalse);
      codec.frame.complete(frame);
      await tester.pump();
      expect(frame.image.debugDisposed, isTrue);
      expect(codec.disposals, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a codec created after retirement is disposed without requesting a frame",
    (tester) async {
      final pendingCodec = Completer<ui.Codec>();
      final source = _SourceImage(pendingCodec.future);
      await tester.pumpWidget(_viewer(source));
      source.retire();
      await source.evict();
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      final codec = _Codec();
      pendingCodec.complete(codec);
      await tester.pump();
      expect(codec.disposals, 1);
      expect(codec.frameRequests, 0);
      expect(tester.takeException(), isNull);
    },
  );
}

Widget _viewer(_SourceImage source, {List<Object>? errors}) => MaterialApp(
  home: Scaffold(
    body: Image(
      image: source,
      errorBuilder: (context, error, stackTrace) {
        errors?.add(error);
        return const Text("source-frame-error");
      },
    ),
  ),
);

class _SourceImage extends ImageProvider<_SourceImage> {
  _SourceImage(this.codec);

  final Future<ui.Codec> codec;
  final _Retirement _retirement = _Retirement();

  void retire() {
    _retirement.retired = true;
  }

  @override
  Future<_SourceImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(
    _SourceImage key,
    ImageDecoderCallback decode,
  ) => createLibrarySourceImageStream(
    codec: codec,
    isRetired: () => _retirement.retired,
    debugLabel: "controlled-frame",
  );
}

class _Retirement {
  bool retired = false;
}

class _Codec implements ui.Codec {
  final Completer<ui.FrameInfo> frame = Completer();
  int frameRequests = 0;
  int disposals = 0;

  @override
  int get frameCount => 1;
  @override
  int get repetitionCount => 0;

  @override
  Future<ui.FrameInfo> getNextFrame() {
    frameRequests += 1;
    return frame.future;
  }

  @override
  void dispose() {
    disposals += 1;
  }
}

Future<ui.FrameInfo> _createFrame(WidgetTester tester) async {
  final frame = await tester.runAsync(() async {
    final codec = await ui.instantiateImageCodec(
      base64Decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      ),
    );
    try {
      return await codec.getNextFrame();
    } finally {
      codec.dispose();
    }
  });
  if (frame == null) {
    throw StateError("The fixture codec produced no frame");
  }
  return frame;
}

Future<void> _pumpRetirement(WidgetTester tester) async {
  for (var frame = 0; frame < 4; frame += 1) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

Future<void> _waitForImage(WidgetTester tester) async {
  for (var attempt = 0; attempt < 20; attempt += 1) {
    await tester.pump(const Duration(milliseconds: 20));
    if (tester
        .widgetList<RawImage>(find.byType(RawImage))
        .any((image) => image.image != null)) {
      return;
    }
  }
  fail("The current guarded image stream did not publish its frame");
}
