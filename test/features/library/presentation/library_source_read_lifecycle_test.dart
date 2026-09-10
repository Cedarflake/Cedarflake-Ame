import "dart:async";
import "dart:convert";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("buffer reads only the approved path and waits for admission", (
    tester,
  ) async {
    final fixture = _Fixture()..reader.gate = Completer();
    await tester.pumpWidget(fixture.viewer("first"));
    await tester.pump();
    expect(fixture.reader.opened, ["first"]);
    expect(fixture.buffers, isEmpty);
    fixture.reader.gate!.complete();
    await tester.pump();
    expect(fixture.buffers.keys, ["approved-first"]);
    await tester.runAsync(() => fixture.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(fixture.reader.leases.single.closed, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "fast navigation retains the active copy and opens only the latest selection",
    (tester) async {
      final fixture = _Fixture();
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      expect(fixture.buffers.keys, ["approved-first"]);
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(fixture.viewer("skipped"));
      await tester.pumpWidget(fixture.viewer("latest"));
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.reader.leases.first.closed, 0);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first", "latest"]);
      expect(fixture.buffers.keys, ["approved-first", "approved-latest"]);
      expect(fixture.reader.leases.first.closed, 1);
      await tester.runAsync(() => fixture.finishBuffer("approved-latest"));
      await _waitForImage(tester);
      expect(find.text("无法打开原图"), findsNothing);
      expect(fixture.reader.leases.last.closed, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "closing during a buffer copy releases only after copy completion",
    (tester) async {
      final fixture = _Fixture();
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.leases.single.closed, 0);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.leases.single.closed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "closing while admission is pending closes its late lease without opening a buffer",
    (tester) async {
      final fixture = _Fixture()..reader.gate = Completer();
      await tester.pumpWidget(fixture.viewer("first"));
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      fixture.reader.gate!.complete();
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.buffers, isEmpty);
      expect(fixture.reader.leases.single.closed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a new selection survives admission completing after old stream retirement",
    (tester) async {
      final fixture = _Fixture()..reader.gate = Completer();
      await tester.pumpWidget(fixture.viewer("first"));
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(fixture.viewer("latest"));
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.buffers, isEmpty);
      fixture.reader.gate!.complete();
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first", "latest"]);
      expect(fixture.reader.leases.first.closed, 1);
      expect(fixture.buffers.keys, ["approved-latest"]);
      await tester.runAsync(() => fixture.finishBuffer("approved-latest"));
      await _waitForImage(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "admission failure is visible and retry performs a fresh guarded read",
    (tester) async {
      final fixture = _Fixture()
        ..reader.failure = const LibrarySourceReadFailure(
          "viewer_source_unavailable",
          "Fixture is unavailable",
        );
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      expect(find.text("无法打开原图"), findsOneWidget);
      expect(fixture.buffers, isEmpty);
      fixture.reader.failure = null;
      await tester.tap(find.text("重试"));
      await tester.pump();
      expect(fixture.reader.opened, ["first", "first"]);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await _waitForImage(tester);
      expect(find.text("无法打开原图"), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("copy failure releases the source before showing retry", (
    tester,
  ) async {
    final fixture = _Fixture();
    await tester.pumpWidget(fixture.viewer("first"));
    await tester.pump();
    fixture.buffers["approved-first"]!.completeError(
      StateError("fixture copy failed"),
    );
    await tester.pump();
    expect(fixture.reader.leases.single.closed, 1);
    expect(find.text("无法打开原图"), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });
}

class _Fixture {
  final _Reader reader = _Reader();
  late final LibrarySourceReadScheduler scheduler = LibrarySourceReadScheduler(
    reader: reader,
  );
  final Map<String, Completer<ui.ImmutableBuffer>> buffers = {};

  Widget viewer(String id) => MaterialApp(
    home: Scaffold(
      body: LibraryViewerImage(
        asset: _asset(id),
        sourceReadScheduler: scheduler,
        sourceBufferLoader: loadBuffer,
      ),
    ),
  );

  Future<ui.ImmutableBuffer> loadBuffer(String path) {
    final completion = Completer<ui.ImmutableBuffer>();
    buffers[path] = completion;
    return completion.future;
  }

  Future<void> finishBuffer(String path) async {
    final buffer = await ui.ImmutableBuffer.fromUint8List(
      base64Decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      ),
    );
    buffers[path]!.complete(buffer);
  }
}

class _Reader implements LibrarySourceReader {
  final List<String> opened = [];
  final List<_Lease> leases = [];
  Completer<void>? gate;
  LibrarySourceReadFailure? failure;

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) async {
    opened.add(asset.locationId);
    await gate?.future;
    final error = failure;
    if (error != null) {
      throw error;
    }
    final lease = _Lease("approved-${asset.locationId}");
    leases.add(lease);
    return lease;
  }
}

class _Lease implements LibrarySourceReadLease {
  _Lease(this.sourcePath);
  @override
  final String sourcePath;
  int closed = 0;

  @override
  Future<void> close() async {
    closed += 1;
  }
}

Future<void> _waitForImage(WidgetTester tester) async {
  for (var attempt = 0; attempt < 100; attempt += 1) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
    if (tester
        .widgetList<RawImage>(find.byType(RawImage))
        .any((image) => image.image != null)) {
      return;
    }
  }
  fail("The guarded viewer did not display its decoded source");
}

ImageStreamCompleter _sourceCompleter(WidgetTester tester) {
  final image = tester
      .widgetList<Image>(find.byType(Image))
      .singleWhere((image) => image.image is LibrarySourceImage);
  final stream = image.image.resolve(ImageConfiguration.empty);
  final completer = stream.completer;
  if (completer == null) {
    throw StateError("The source image stream has not been admitted");
  }
  return completer;
}

Future<void> _pumpRetirement(WidgetTester tester) async {
  for (var frame = 0; frame < 4; frame += 1) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

LibraryAsset _asset(String id) => LibraryAsset(
  assetId: id,
  locationId: id,
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unapproved-$id",
  displayPath: "$id.png",
  relativePath: "$id.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 1,
  height: 1,
);
