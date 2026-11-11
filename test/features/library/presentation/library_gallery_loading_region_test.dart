import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_loading_region.dart";
import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_rail_presentation.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("content waiting owns feedback before either quiet interval", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final controller = ScrollController();
    Future<void> show(bool contentWaiting) => tester.pumpWidget(
      MaterialApp(
        home: LibraryGalleryLoadingRegion(
          isLoading: contentWaiting,
          showContentIndicator: contentWaiting,
          child: Row(
            children: [
              const Expanded(
                child: SizedBox.expand(key: Key("retained-content")),
              ),
              LibraryTimeRailPresentation(
                timeline: null,
                layoutShape: GalleryLayoutShape.equalHeight,
                scrollController: controller,
                frame: null,
                navigationProjection: null,
                isLoading: true,
                onChangeStart: (_) {},
                onChanged: null,
                onChangeEnd: null,
                onStep: null,
                onInteractionCancelled: () {},
              ),
            ],
          ),
        ),
      ),
    );
    try {
      await show(true);
      final bounds = tester.getRect(find.byKey(const Key("retained-content")));
      expect(find.byType(CircularProgressIndicator), findsNothing);
      await tester.pump(const Duration(milliseconds: 249));
      expect(find.semantics.byLabel("正在加载图片"), findsNothing);
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump(const Duration(seconds: 1));
      expect(find.semantics.byLabel("正在加载图片"), findsOneWidget);
      expect(find.byType(AmeLoadingIndicator), findsOneWidget);
      await show(false);
      await tester.pump(const Duration(seconds: 1));
      expect(find.semantics.byLabel("正在加载图片"), findsNothing);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(tester.getRect(find.byKey(const Key("retained-content"))), bounds);
      await show(true);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
    } finally {
      controller.dispose();
      semantics.dispose();
    }
  });

  testWidgets(
    "short refreshes stay quiet and delayed feedback retires with its owner",
    (tester) async {
      Future<void> show(bool loading, {Duration? delay}) => tester.pumpWidget(
        MaterialApp(
          home: LibraryGalleryLoadingRegion(
            isLoading: loading,
            delay: delay ?? const Duration(milliseconds: 250),
            child: const SizedBox.expand(),
          ),
        ),
      );
      await show(false);
      await show(true);
      await tester.pump(const Duration(milliseconds: 100));
      expect(_opacity(tester), 0);
      await show(false);
      await tester.pump(const Duration(milliseconds: 300));
      expect(_opacity(tester), 0);
      await show(true);
      await tester.pump(const Duration(milliseconds: 200));
      await show(true, delay: Duration.zero);
      await tester.pump(const Duration(milliseconds: 49));
      expect(_opacity(tester), 0);
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump(const Duration(milliseconds: 100));
      expect(_opacity(tester), greaterThan(0));
      await tester.pump(const Duration(seconds: 1));
      expect(_opacity(tester), 1);
      await show(false);
      await tester.pump(const Duration(milliseconds: 30));
      final fading = _opacity(tester);
      expect(fading, inExclusiveRange(0, 1));
      await show(true);
      expect(_opacity(tester), fading);
      await tester.pump(const Duration(milliseconds: 100));
      expect(_opacity(tester), greaterThan(fading));
      await show(false);
      await tester.pump(const Duration(seconds: 1));
      expect(_opacity(tester), 0);
      await show(true);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump(const Duration(milliseconds: 300));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("loading line preserves geometry and passes gallery input", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    var taps = 0;
    Future<void> show(bool isLoading) => tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: SizedBox(
            width: 300,
            height: 200,
            child: LibraryGalleryLoadingRegion(
              isLoading: isLoading,
              child: GestureDetector(
                onTap: () => taps++,
                behavior: HitTestBehavior.opaque,
                child: const SizedBox.expand(key: Key("gallery-content")),
              ),
            ),
          ),
        ),
      ),
    );
    final content = find.byKey(const Key("gallery-content"));
    final indicator = find.byType(LinearProgressIndicator);
    try {
      await show(false);
      final idleBounds = tester.getRect(content);
      expect(_opacity(tester), 0);

      await show(true);
      await tester.pump(const Duration(milliseconds: 250));
      await tester.pump(const Duration(seconds: 1));
      expect(tester.getRect(content), idleBounds);
      expect(tester.getSize(indicator).height, 2);
      expect(tester.getTopLeft(indicator), idleBounds.topLeft);
      expect(tester.getSemantics(indicator).label, "正在加载图片");
      await tester.tapAt(idleBounds.topLeft + const Offset(10, 1));
      expect(taps, 1);

      await show(false);
      expect(tester.getRect(content), idleBounds);
      await tester.pump(const Duration(seconds: 1));
      expect(_opacity(tester), 0);
      expect(find.semantics.byLabel("正在加载图片"), findsNothing);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets("reduced motion keeps the quiet interval without fading", (
    tester,
  ) async {
    Future<void> show(bool loading) => tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: LibraryGalleryLoadingRegion(
            isLoading: loading,
            child: const SizedBox.expand(),
          ),
        ),
      ),
    );
    await show(true);
    await tester.pump(const Duration(milliseconds: 249));
    expect(_opacity(tester), 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(_opacity(tester), 1);
    await show(false);
    expect(_opacity(tester), 0);
  });
}

double _opacity(WidgetTester tester) => tester
    .widget<FadeTransition>(
      find.descendant(
        of: find.byKey(const Key("library-loading-feedback")),
        matching: find.byType(FadeTransition),
      ),
    )
    .opacity
    .value;
