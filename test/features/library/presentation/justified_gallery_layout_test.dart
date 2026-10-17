import "package:cedarflake_ame/features/library/presentation/widgets/justified_gallery_layout.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("keeps every photo row at the selected fixed height", () {
    const availableWidth = 1146.0;
    const spacing = 6.0;
    const layout = JustifiedGalleryLayout(
      targetRowHeight: 138,
      spacing: spacing,
    );

    final rows = layout.compute(
      aspectRatios: const [
        1,
        1.42,
        1.78,
        0.82,
        1.25,
        0.72,
        1,
        1.42,
        1.78,
        0.82,
        1.25,
      ],
      availableWidth: availableWidth,
    );

    expect(rows, hasLength(2));
    expect(rows.every((row) => row.height == 138), isTrue);
    final completeRow = rows.first;
    final completeRowWidth =
        completeRow.cells.fold<double>(0, (sum, cell) => sum + cell.width) +
        spacing * (completeRow.cells.length - 1);
    expect(completeRow.isJustified, isTrue);
    expect(completeRowWidth, closeTo(availableWidth, 0.001));

    final sparseRow = rows.last;
    final sparseRowWidth =
        sparseRow.cells.fold<double>(0, (sum, cell) => sum + cell.width) +
        spacing * (sparseRow.cells.length - 1);
    expect(sparseRow.isJustified, isFalse);
    expect(sparseRowWidth, lessThan(availableWidth));
  });

  test("keeps a sparse final row at natural width", () {
    const layout = JustifiedGalleryLayout(targetRowHeight: 138, spacing: 6);

    final rows = layout.compute(
      aspectRatios: const [1.42],
      availableWidth: 1146,
    );

    expect(rows.single.isJustified, isFalse);
    expect(rows.single.height, 138);
    expect(rows.single.cells.single.width, closeTo(195.96, 0.001));
  });

  test("keeps extreme portrait cells wide enough for tile actions", () {
    const layout = JustifiedGalleryLayout(targetRowHeight: 96, spacing: 6);

    final rows = layout.compute(
      aspectRatios: const [0.01],
      availableWidth: 1146,
    );

    expect(rows.single.height, 96);
    expect(rows.single.cells.single.width, 128);
  });

  test("filled rows and narrow viewports respect the card width floor", () {
    const layout = JustifiedGalleryLayout(targetRowHeight: 96, spacing: 6);
    for (final width in [100.0, 600.0, 1146.0]) {
      final rows = layout.compute(
        aspectRatios: List.filled(30, 0.01),
        availableWidth: width,
      );
      expect(
        rows.expand((row) => row.cells).map((cell) => cell.width),
        everyElement(greaterThanOrEqualTo(width < 128 ? width : 128)),
      );
      for (final row in rows.where((row) => row.isJustified)) {
        final occupied =
            row.cells.fold(0.0, (sum, cell) => sum + cell.width) +
            6 * (row.cells.length - 1);
        expect(occupied, closeTo(width, 0.001));
      }
    }
  });
}
