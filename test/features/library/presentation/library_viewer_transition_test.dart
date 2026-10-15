import "dart:async";

import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_transition.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets(
    "overlay semantics remain reachable through zero opacity and interrupted exit",
    (tester) async {
      final semantics = tester.ensureSemantics();
      final gallery = Column(
        children: [
          const Text("gallery"),
          Slider(value: 0.5, onChanged: (_) {}),
        ],
      );
      final viewer = Column(
        children: [
          const Text("viewer"),
          Slider(value: 0.5, onChanged: (_) {}),
        ],
      );
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer first transparent frame",
      );
      await tester.pump(const Duration(milliseconds: 200));
      await tester.pumpWidget(_host(gallery, null));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer exit input retired",
      );
      await tester.pump(const Duration(milliseconds: 30));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      await tester.pumpWidget(_host(gallery, null));
      await tester.pump(const Duration(milliseconds: 120));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer retired",
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      semantics.dispose();
    },
  );
  testWidgets(
    "return restores gallery position and input before the fade finishes",
    (tester) async {
      final scroll = ScrollController(initialScrollOffset: 200);
      addTearDown(scroll.dispose);
      var galleryTaps = 0;
      var viewerTaps = 0;
      final gallery = ListView(
        controller: scroll,
        children: [
          const SizedBox(height: 200),
          TextButton(
            key: const Key("gallery-action"),
            onPressed: () => galleryTaps++,
            child: const Text("gallery"),
          ),
          const SizedBox(height: 1000),
        ],
      );
      final viewer = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => viewerTaps++,
        child: const Center(child: Text("viewer")),
      );
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      expect(scroll.offset, 200);
      await tester.pumpWidget(_host(gallery, null));
      expect(find.text("viewer"), findsOneWidget);
      expect(
        LibraryViewerActivity.isActiveOf(tester.element(find.text("viewer"))),
        isFalse,
      );
      await tester.tap(find.byKey(const Key("gallery-action")));
      expect(galleryTaps, 1);
      expect(viewerTaps, 0);
      expect(scroll.offset, 200);
      await tester.pump(const Duration(milliseconds: 120));
      expect(find.text("viewer"), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    "quick reopen keeps one active viewer and ignores an older exit",
    (tester) async {
      const gallery = Text("gallery");
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, const Text("first")));
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pumpWidget(_host(gallery, null));
      await tester.pump(const Duration(milliseconds: 20));
      await tester.pumpWidget(_host(gallery, const Text("second")));
      expect(find.text("first"), findsNothing);
      expect(find.text("second"), findsOneWidget);
      expect(
        LibraryViewerActivity.isActiveOf(tester.element(find.text("second"))),
        isTrue,
      );
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.text("second"), findsOneWidget);
      expect(
        tester
            .widget<FadeTransition>(find.byType(FadeTransition).last)
            .opacity
            .value,
        1,
      );
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("reduced motion removes the closing viewer in the same frame", (
    tester,
  ) async {
    const gallery = Text("gallery");
    await tester.pumpWidget(_host(gallery, null, reducedMotion: true));
    await tester.pumpWidget(
      _host(gallery, const Text("viewer"), reducedMotion: true),
    );
    expect(find.text("viewer"), findsOneWidget);
    await tester.pumpWidget(_host(gallery, null, reducedMotion: true));
    expect(find.text("viewer"), findsNothing);
    expect(find.text("gallery"), findsOneWidget);
    expect(tester.binding.transientCallbackCount, 0);
  });

  testWidgets(
    "closing retires a pending source before the visual exit completes",
    (tester) async {
      final reader = _DelayedReader();
      final scheduler = LibrarySourceReadScheduler(reader: reader);
      var bufferReads = 0;
      final viewer = LibraryViewerImage(
        asset: _asset(),
        sourceReadScheduler: scheduler,
        sourceBufferLoader: (_) async {
          bufferReads++;
          throw StateError("A retired source must not be decoded");
        },
      );
      const gallery = Text("gallery");
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      expect(reader.requests, 1);
      await tester.pumpWidget(_host(gallery, null));
      expect(find.byType(LibraryViewerImage), findsOneWidget);
      final lease = _Lease();
      reader.result.complete(lease);
      await tester.pump();
      expect(lease.closes, 1);
      expect(bufferReads, 0);
      await tester.pump(const Duration(milliseconds: 120));
      expect(find.byType(LibraryViewerImage), findsNothing);
      expect(tester.takeException(), isNull);
      PaintingBinding.instance.imageCache.clear();
      PaintingBinding.instance.imageCache.clearLiveImages();
    },
  );
}

Widget _host(Widget gallery, Widget? viewer, {bool reducedMotion = false}) =>
    MaterialApp(
      home: MediaQuery(
        data: MediaQueryData(disableAnimations: reducedMotion),
        child: Scaffold(
          body: LibraryViewerTransition(gallery: gallery, viewer: viewer),
        ),
      ),
    );

LibraryAsset _asset() => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unopened-source.png",
  displayPath: "unopened-source.png",
  relativePath: "unopened-source.png",
  previewPath: "",
  fileSize: BigInt.from(256),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 200,
  height: 100,
  previewStatus: LibraryPreviewStatus.pending,
);

class _DelayedReader implements LibrarySourceReader {
  final result = Completer<LibrarySourceReadLease>();
  var requests = 0;

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) {
    requests++;
    return result.future;
  }
}

class _Lease implements LibrarySourceReadLease {
  var closes = 0;

  @override
  String get sourcePath => "unopened-source.png";

  @override
  Future<void> close() async {
    closes++;
  }
}
