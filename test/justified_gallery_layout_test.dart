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
      aspectRatios: const [0.72],
      availableWidth: 1146,
    );

    expect(rows.single.isJustified, isFalse);
    expect(rows.single.height, 138);
    expect(rows.single.cells.single.width, closeTo(99.36, 0.001));
  });
}
