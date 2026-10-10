import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_prepend_compensation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "prepend preserves later input when the trailing window is trimmed",
    (tester) async {
      final position = await _position(tester);
      position.jumpTo(300);
      final compensation = _capture(position);
      position.jumpTo(500);

      // Both windows have the same total extent after trailing rows are retired.
      expect(
        compensation.resolve(
          position: position,
          metrics: _metrics(start: 50),
          geometry: _geometry(start: 50),
          revision: BigInt.one,
        ),
        1000,
      );
    },
  );

  testWidgets("manifest takeover retires pending window compensation", (
    tester,
  ) async {
    final position = await _position(tester);
    final compensation = _capture(position);
    expect(
      compensation.resolve(
        position: position,
        metrics: _metrics(start: 0, queryWide: true),
        geometry: _geometry(start: 50),
        revision: BigInt.one,
      ),
      isNull,
    );
    expect(
      LibraryGalleryPrependCompensation.capture(
        position: position,
        metrics: _metrics(start: 0, queryWide: true),
        geometry: _geometry(start: 100),
        revision: BigInt.one,
      ).resolve(
        position: position,
        metrics: _metrics(start: 50),
        geometry: _geometry(start: 50),
        revision: BigInt.one,
      ),
      isNull,
    );
  });

  testWidgets(
    "new position intent retires same-query overlapping compensation",
    (tester) async {
      final position = await _position(tester);
      final compensation = _capture(position);
      compensation.retire();
      expect(compensation.isActive, isFalse);
      expect(
        compensation.resolve(
          position: position,
          metrics: _metrics(start: 50),
          geometry: _geometry(start: 50),
          revision: BigInt.one,
        ),
        isNull,
      );
    },
  );

  testWidgets(
    "replacement query revision and detached scroll positions reject old compensation",
    (tester) async {
      final position = await _position(tester);
      final compensation = _capture(position);
      for (final context in [
        (queryId: "other", revision: BigInt.one, start: 50),
        (queryId: "query-1", revision: BigInt.two, start: 50),
        (queryId: "query-1", revision: BigInt.one, start: 150),
      ]) {
        expect(
          compensation.resolve(
            position: position,
            metrics: _metrics(start: context.start),
            geometry: _geometry(start: context.start, queryId: context.queryId),
            revision: context.revision,
          ),
          isNull,
        );
      }
      await tester.pumpWidget(const SizedBox.shrink());
      final replacement = await _position(tester);
      expect(
        compensation.resolve(
          position: replacement,
          metrics: _metrics(start: 50),
          geometry: _geometry(start: 50),
          revision: BigInt.one,
        ),
        isNull,
      );
    },
  );
}

LibraryGalleryPrependCompensation _capture(ScrollPosition position) =>
    LibraryGalleryPrependCompensation.capture(
      position: position,
      metrics: _metrics(start: 100),
      geometry: _geometry(start: 100),
      revision: BigInt.one,
    );

Future<ScrollPosition> _position(WidgetTester tester) async {
  await tester.pumpWidget(
    const MaterialApp(
      home: SingleChildScrollView(child: SizedBox(height: 5000)),
    ),
  );
  return tester.state<ScrollableState>(find.byType(Scrollable)).position;
}

LibraryGalleryLayoutMetrics _metrics({
  required int start,
  bool queryWide = false,
}) => LibraryGalleryLayoutMetrics(
  contentExtent: 2050,
  photoRowHeight: 10,
  dateAnchors: const [],
  locationOffsets: const {},
  itemOffsets: [for (var index = 0; index < 200; index += 1) 50.0 + index * 10],
  itemIndexBase: start,
  isQueryWide: queryWide,
);

LibraryVirtualGalleryGeometry _geometry({
  required int start,
  String queryId = "query-1",
}) => LibraryVirtualGalleryGeometry(
  totalContentExtent: 2050,
  viewportExtent: 600,
  leadingExtent: 0,
  loadedContentExtent: 2050,
  trailingExtent: 0,
  windowStartItemOffset: start,
  loadedItemCount: 200,
  totalItemCount: 2000,
  queryId: queryId,
);
