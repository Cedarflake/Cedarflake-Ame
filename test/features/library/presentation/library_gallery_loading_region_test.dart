import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_loading_region.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
      expect(indicator, findsNothing);

      await show(true);
      expect(tester.getRect(content), idleBounds);
      expect(tester.getSize(indicator).height, 2);
      expect(tester.getTopLeft(indicator), idleBounds.topLeft);
      expect(tester.getSemantics(indicator).label, "正在加载图片");
      await tester.tapAt(idleBounds.topLeft + const Offset(10, 1));
      expect(taps, 1);

      await show(false);
      expect(tester.getRect(content), idleBounds);
      expect(indicator, findsNothing);
      expect(find.bySemanticsLabel("正在加载图片"), findsNothing);
    } finally {
      semantics.dispose();
    }
  });
}
