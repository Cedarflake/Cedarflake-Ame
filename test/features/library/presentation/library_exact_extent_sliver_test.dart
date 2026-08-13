import "dart:typed_data";

import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_extent_sliver.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  const tolerance = 0.000001;

  testWidgets(
    "keeps exact range and position across deep jumps and content rebuilds",
    (tester) async {
      const entryCount = 17170;
      const viewportHeight = 640.0;
      const topPadding = 18.0;
      const bottomPadding = 72.0;
      final geometry = _buildGeometry(entryCount);
      final generation = ValueNotifier(0);
      final controller = ScrollController();
      final builtIndices = <int>{};
      addTearDown(() async {
        controller.dispose();
        generation.dispose();
        await tester.binding.setSurfaceSize(null);
      });
      await tester.binding.setSurfaceSize(const Size(1000, viewportHeight));

      await tester.pumpWidget(
        MaterialApp(
          home: ValueListenableBuilder<int>(
            valueListenable: generation,
            builder: (context, value, child) {
              return CustomScrollView(
                controller: controller,
                slivers: [
                  SliverPadding(
                    padding: const EdgeInsets.only(
                      top: topPadding,
                      bottom: bottomPadding,
                    ),
                    sliver: LibraryExactExtentSliver.builder(
                      itemStartOffsets: geometry.offsets,
                      contentExtent: geometry.contentExtent,
                      addSemanticIndexes: false,
                      itemBuilder: (context, index) {
                        builtIndices.add(index);
                        return SizedBox(
                          key: ValueKey("exact-entry-$index"),
                          child: Text("$index-$value"),
                        );
                      },
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      );

      final expectedMaxExtent =
          geometry.contentExtent + topPadding + bottomPadding - viewportHeight;
      expect(
        controller.position.maxScrollExtent,
        closeTo(expectedMaxExtent, tolerance),
      );

      builtIndices.clear();
      final deepOffset = expectedMaxExtent * 0.93;
      controller.jumpTo(deepOffset);
      await tester.pump();

      expect(controller.offset, closeTo(deepOffset, tolerance));
      expect(builtIndices, isNotEmpty);
      expect(
        builtIndices.reduce((left, right) => left < right ? left : right),
        greaterThan(entryCount * 0.8),
      );
      expect(builtIndices.length, lessThan(100));

      final stableOffset = expectedMaxExtent * 0.71;
      controller.jumpTo(stableOffset);
      await tester.pump();
      final maxExtentBeforeRebuild = controller.position.maxScrollExtent;
      final pixelsBeforeRebuild = controller.position.pixels;

      generation.value = 1;
      await tester.pump();

      expect(
        controller.position.maxScrollExtent,
        closeTo(maxExtentBeforeRebuild, tolerance),
      );
      expect(
        controller.position.pixels,
        closeTo(pixelsBeforeRebuild, tolerance),
      );

      controller.jumpTo(controller.position.maxScrollExtent);
      await tester.pump();

      expect(find.byKey(const ValueKey("exact-entry-17169")), findsOneWidget);
      expect(
        controller.position.maxScrollExtent,
        closeTo(expectedMaxExtent, tolerance),
      );
    },
  );

  testWidgets("applies a layout correction once across preview-only rebuilds", (
    tester,
  ) async {
    final geometry = _buildGeometry(500);
    final previewGeneration = ValueNotifier(0);
    final correction = ValueNotifier<LibraryExactExtentLayoutCorrection?>(null);
    final appliedGenerations = <Object>[];
    final controller = ScrollController();
    addTearDown(() async {
      controller.dispose();
      previewGeneration.dispose();
      correction.dispose();
      await tester.binding.setSurfaceSize(null);
    });
    await tester.binding.setSurfaceSize(const Size(1000, 640));

    await tester.pumpWidget(
      MaterialApp(
        home: ListenableBuilder(
          listenable: Listenable.merge([previewGeneration, correction]),
          builder: (context, child) {
            return CustomScrollView(
              controller: controller,
              slivers: [
                LibraryExactExtentSliver.builder(
                  itemStartOffsets: geometry.offsets,
                  contentExtent: geometry.contentExtent,
                  layoutCorrection: correction.value,
                  onLayoutCorrectionApplied: appliedGenerations.add,
                  itemBuilder: (context, index) => SizedBox(
                    key: ValueKey("corrected-entry-$index"),
                    child: Text("$index-${previewGeneration.value}"),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );

    controller.jumpTo(20000);
    await tester.pump();
    correction.value = const LibraryExactExtentLayoutCorrection(
      generation: 1,
      delta: 250,
    );
    await tester.pump();

    final renderSliver = tester.renderObject<RenderLibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    final pixelsAfterCorrection = controller.position.pixels;
    final maximumAfterCorrection = controller.position.maxScrollExtent;
    final visibleEntry = find.textContaining("-0").first;
    final rectAfterCorrection = tester.getRect(visibleEntry);
    expect(pixelsAfterCorrection, closeTo(20250, tolerance));
    expect(renderSliver.appliedLayoutCorrectionCount, 1);
    expect(appliedGenerations, [1]);

    previewGeneration.value = 1;
    await tester.pump();

    expect(
      controller.position.pixels,
      closeTo(pixelsAfterCorrection, tolerance),
    );
    expect(
      controller.position.maxScrollExtent,
      closeTo(maximumAfterCorrection, tolerance),
    );
    expect(
      tester.getRect(find.textContaining("-1").first),
      rectAfterCorrection,
    );
    expect(renderSliver.appliedLayoutCorrectionCount, 1);
    expect(appliedGenerations, [1]);
    expect(identical(renderSliver.itemStartOffsets, geometry.offsets), isTrue);
  });

  testWidgets("keeps transition and resize correction generations distinct", (
    tester,
  ) async {
    final geometry = _buildGeometry(500);
    final correction = ValueNotifier<LibraryExactExtentLayoutCorrection?>(null);
    final controller = ScrollController();
    addTearDown(() async {
      controller.dispose();
      correction.dispose();
      await tester.binding.setSurfaceSize(null);
    });
    await tester.binding.setSurfaceSize(const Size(1000, 640));

    await tester.pumpWidget(
      MaterialApp(
        home: ValueListenableBuilder(
          valueListenable: correction,
          builder: (context, value, child) => CustomScrollView(
            controller: controller,
            slivers: [
              LibraryExactExtentSliver.builder(
                itemStartOffsets: geometry.offsets,
                contentExtent: geometry.contentExtent,
                layoutCorrection: value,
                itemBuilder: (context, index) =>
                    SizedBox(key: ValueKey("scoped-entry-$index")),
              ),
            ],
          ),
        ),
      ),
    );
    controller.jumpTo(20000);
    await tester.pump();

    correction.value = const LibraryExactExtentLayoutCorrection(
      generation: 1,
      delta: 100,
    );
    await tester.pump();
    correction.value = const LibraryExactExtentLayoutCorrection(
      generation: (scope: "gallery-transition", value: 1),
      delta: 100,
    );
    await tester.pump();

    final renderSliver = tester.renderObject<RenderLibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    expect(controller.position.pixels, closeTo(20200, tolerance));
    expect(renderSliver.appliedLayoutCorrectionCount, 2);
  });
}

({Float64List offsets, double contentExtent}) _buildGeometry(int itemCount) {
  final offsets = Float64List(itemCount);
  var contentExtent = 0.0;
  for (var index = 0; index < itemCount; index++) {
    offsets[index] = contentExtent;
    contentExtent += 72 + ((index * 37) % 113);
  }
  return (offsets: offsets, contentExtent: contentExtent);
}
