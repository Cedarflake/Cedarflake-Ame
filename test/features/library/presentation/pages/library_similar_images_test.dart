import "dart:async";
import "dart:convert";
import "dart:math" as math;
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_similar_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_similar_review_controller.dart";
import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/application/ports/library_source_reader.dart";
import "package:cedarflake_ame/features/library/application/ports/library_similar_analysis.dart";
import "package:cedarflake_ame/features/library/application/library_workbench_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_analysis.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_review.dart";
import "package:cedarflake_ame/features/library/presentation/pages/library_duplicates_page.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_cards.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_comparison.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_similar_review_cards.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_source_scope.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter/gestures.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../support/application_test_bindings.dart";
import "../../../../support/duplicate_review_fixtures.dart";
import "../../../../support/similar_review_fixtures.dart";

void main() {
  testWidgets(
    "running results join ignored groups and preserve comparison until navigation",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1100, 850));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final exact = ReviewFixtureService()
        ..decision = const LibraryDuplicateDecision(ignored: true);
      final similar = SimilarReviewFixture()
        ..decision = LibrarySimilarDecision(
          revision: BigInt.one,
          ignored: true,
        );
      final library = ReviewFixtureLibrary();
      final source = _ComparisonReader();
      final analysisService = _AnalysisService();
      final members = List.of(similar.members);
      similar.members.clear();
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          libraryControllerProvider.overrideWith(() => library),
          libraryDuplicateReviewProvider.overrideWithValue(exact),
          librarySimilarReviewProvider.overrideWithValue(similar),
          librarySimilarAnalysisProvider.overrideWithValue(analysisService),
          librarySourceReaderProvider.overrideWithValue(source),
        ]),
      );
      addTearDown(container.dispose);
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.duplicates);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            builder: (context, child) => LibraryViewerSourceScope(
              scheduler: container.read(librarySourceReadSchedulerProvider),
              bufferLoader: source.loadBuffer,
              child: child!,
            ),
            home: const Scaffold(body: LibraryDuplicatesPage()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      Future<void> chooseView(String label) async {
        await tester.tap(find.byKey(const Key("duplicate-group-menu")));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label).last);
        await tester.pumpAndSettle();
      }

      Future<void> back() async {
        await tester.tap(find.byKey(const Key("duplicate-review-back")));
        await tester.pumpAndSettle();
      }

      await chooseView("已忽略");
      expect(find.text("1 个组"), findsOneWidget);
      expect(find.byType(LibrarySimilarGroupCard), findsNothing);
      similar.members.addAll(members);
      final pendingGroups = Completer<LibrarySimilarGroups>();
      similar.pendingGroups = pendingGroups;
      final analysis = container.read(librarySimilarAnalysisControllerProvider);
      analysis.start();
      await tester.pump();
      final initialQueries = similar.queries.length;
      await tester.pump(const Duration(seconds: 2));
      await tester.pump(const Duration(seconds: 6));
      expect(similar.queries.length, initialQueries + 1);
      pendingGroups.complete(
        LibrarySimilarGroups(groups: [similar.group()], total: 1),
      );
      similar.pendingGroups = null;
      await tester.pumpAndSettle();
      expect(analysis.isRunning, isTrue);
      expect(find.text("2 个组"), findsOneWidget);
      expect(find.byType(LibraryDuplicateGroupCard), findsOneWidget);
      expect(find.byType(LibrarySimilarGroupCard), findsOneWidget);
      await analysis.close();
      await tester.pumpAndSettle();
      final stoppedQueries = similar.queries.length;
      await tester.pump(const Duration(seconds: 4));
      expect(similar.queries.length, stoppedQueries);
      expect(find.textContaining("重复组"), findsOneWidget);
      expect(find.textContaining("相似组"), findsOneWidget);
      await tester.tap(find.text("查看相似图片"));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("duplicate-group-menu")), findsNothing);
      expect(
        find.byKey(const Key("duplicate-similarity-degree")),
        findsNothing,
      );
      await tester.tap(find.text("恢复此组"));
      await tester.pumpAndSettle();
      expect(similar.decision.ignored, isFalse);
      expect(find.byType(LibrarySimilarMemberCard), findsNWidgets(2));
      await back();
      expect(find.text("1 个组"), findsOneWidget);
      expect(find.byType(LibrarySimilarGroupCard), findsNothing);

      await chooseView("相似组");
      expect(similar.queries.last.degree, LibrarySimilarityDegree.standard);
      await tester.tap(find.byKey(const Key("duplicate-similarity-degree")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("严格"));
      await tester.pumpAndSettle();
      expect(similar.queries.last.degree, LibrarySimilarityDegree.strict);
      await tester.tap(find.text("查看相似图片"));
      await tester.pumpAndSettle();
      expect(find.bySemanticsLabel("选择图片"), findsNWidgets(2));
      await tester.tap(find.byKey(const ValueKey("prefer-one")));
      await tester.pumpAndSettle();
      expect(similar.members.first.preferred, isTrue);
      await tester.tap(find.byKey(const ValueKey("prefer-two")));
      await tester.pumpAndSettle();
      for (final id in ["one", "two"]) {
        expect(
          tester
              .widget<IconButton>(find.byKey(ValueKey("prefer-$id")))
              .isSelected,
          isTrue,
        );
      }
      expect(find.byIcon(Symbols.bookmark_rounded), findsNWidgets(2));
      expect(find.bySemanticsLabel("取消选择"), findsNWidgets(2));
      await tester.tap(find.byKey(const ValueKey("prefer-one")));
      await tester.pumpAndSettle();
      expect(similar.members.first.preferred, isFalse);
      expect(similar.members.last.preferred, isTrue);
      expect(library.refreshes, 0);
      for (final id in ["one", "two"]) {
        await tester.tap(find.byKey(ValueKey("compare-select-$id")));
        await tester.pump();
      }
      expect(
        tester.getCenter(find.text("对比图片")).dy,
        tester.getCenter(find.text("忽略此组")).dy,
      );
      await tester.tap(find.text("对比图片"));
      await tester.pump();
      for (var attempt = 0; attempt < 100; attempt++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 10)),
        );
        await tester.pump();
        final frames = tester.widgetList<RawImage>(find.byType(RawImage));
        if (frames.where((image) => image.image != null).length == 2) break;
      }
      expect(
        tester
            .widgetList<RawImage>(find.byType(RawImage))
            .where((image) => image.image != null),
        hasLength(2),
      );
      expect(find.byKey(const Key("viewer-image-loading")), findsNothing);
      expect(source.opened, unorderedEquals(["one", "two"]));
      expect(source.closed, unorderedEquals(["one", "two"]));
      await tester.pumpAndSettle();
      expect(find.byType(LibraryImageComparison), findsOneWidget);
      expect(find.byType(LibraryViewerImage), findsNWidgets(2));
      for (final key in [
        "window-minimize",
        "window-maximize",
        "window-close",
        "viewer-window-drag-region",
      ]) {
        expect(find.byKey(Key(key)), findsOneWidget);
      }
      expect(find.text("100%"), findsOneWidget);
      final toolbar = tester.getRect(
        find.byKey(const Key("viewer-zoom-controls")),
      );
      final linked = tester.getRect(find.byKey(const Key("comparison-link")));
      final reset = tester.getRect(find.byKey(const Key("comparison-reset")));
      final mode = tester.getRect(find.byKey(const Key("viewer-zoom-mode")));
      expect(linked.size, reset.size);
      expect(reset.size, mode.size);
      expect(
        reset.center.dx - linked.center.dx,
        mode.center.dx - reset.center.dx,
      );
      for (final action in [
        find.byKey(const Key("comparison-link")),
        find.byKey(const Key("comparison-reset")),
        find.byKey(const Key("viewer-zoom-in")),
      ]) {
        expect(tester.getRect(action).top, greaterThanOrEqualTo(toolbar.top));
      }
      await tester.tap(find.byKey(const Key("comparison-reset")));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("viewer-zoom-in")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 90));
      final intermediate = tester
          .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
          .map((value) => value.zoomScale)
          .toList();
      expect(intermediate.first, greaterThan(1));
      expect(intermediate.first, lessThan(1.25));
      expect(intermediate.last, closeTo(intermediate.first, 0.001));
      await tester.pumpAndSettle();
      expect(
        tester
            .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
            .map((value) => value.zoomScale),
        [1.25, 1.25],
      );
      await tester.tap(find.byKey(const Key("comparison-link")));
      await tester.tap(find.byKey(const Key("viewer-zoom-in")));
      await tester.pumpAndSettle();
      expect(
        tester
            .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
            .map((value) => value.zoomScale),
        [1.5625, 1.25],
      );
      final rightPane = find.byKey(const ValueKey("comparison-image-right"));
      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: tester.getCenter(rightPane),
          scrollDelta: const Offset(0, -24),
          kind: PointerDeviceKind.mouse,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 90));
      final wheelMiddle = tester
          .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
          .last
          .zoomScale;
      final wheelTarget = 1.25 * math.exp(24 / 180);
      expect(wheelMiddle, greaterThan(1.25));
      expect(wheelMiddle, lessThan(wheelTarget));
      await tester.pumpAndSettle();
      final settled = tester
          .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
          .toList();
      expect(settled.first.zoomScale, closeTo(1.5625, 0.001));
      expect(settled.last.zoomScale, closeTo(wheelTarget, 0.001));
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.pump();
      await tester.tap(find.byKey(const Key("viewer-zoom-in")));
      await tester.pumpAndSettle();
      final keyboardSelected = tester
          .widgetList<LibraryViewerImage>(find.byType(LibraryViewerImage))
          .toList();
      expect(keyboardSelected.first.zoomScale, closeTo(1.5625 * 1.25, 0.001));
      expect(keyboardSelected.last.zoomScale, closeTo(wheelTarget, 0.001));
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(find.byType(LibrarySimilarMemberCard), findsNWidgets(2));
      expect(
        tester
            .widget<Checkbox>(find.byKey(const ValueKey("compare-select-one")))
            .value,
        isTrue,
      );
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.gallery);
      await tester.pumpAndSettle();
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.duplicates);
      await tester.pumpAndSettle();
      expect(find.byType(LibrarySimilarGroupCard), findsOneWidget);
      await tester.tap(find.text("查看相似图片"));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<Checkbox>(find.byKey(const ValueKey("compare-select-one")))
            .value,
        isFalse,
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(library.demands, isEmpty);
    },
  );
}

class _AnalysisService implements LibrarySimilarAnalysis {
  @override
  Future<LibrarySimilarAnalysisSession> open(String runId) async =>
      _AnalysisSession(runId);
}

class _AnalysisSession implements LibrarySimilarAnalysisSession {
  _AnalysisSession(this.runId);
  final String runId;
  final completion = Completer<LibrarySimilarAnalysisProgress>();

  @override
  Future<LibrarySimilarAnalysisProgress> run() => completion.future;

  @override
  LibrarySimilarAnalysisProgress progress() => LibrarySimilarAnalysisProgress(
    runId: runId,
    phase: LibrarySimilarAnalysisPhase.running,
  );

  @override
  void cancel() => completion.complete(
    LibrarySimilarAnalysisProgress(
      runId: runId,
      phase: LibrarySimilarAnalysisPhase.cancelled,
    ),
  );

  @override
  void suspend(bool suspended) {}

  @override
  void dispose() {}
}

class _ComparisonReader implements LibrarySourceReader {
  final List<String> opened = [];
  final List<String> closed = [];

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) async {
    opened.add(asset.locationId);
    return _ComparisonLease(asset.locationId, closed.add);
  }

  Future<ui.ImmutableBuffer> loadBuffer(
    String path,
  ) => ui.ImmutableBuffer.fromUint8List(
    base64Decode(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
    ),
  );
}

class _ComparisonLease implements LibrarySourceReadLease {
  _ComparisonLease(this.sourcePath, this.onClose);

  @override
  final String sourcePath;
  final void Function(String) onClose;

  @override
  Future<void> close() async => onClose(sourcePath);
}
