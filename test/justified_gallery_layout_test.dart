import "package:cedarflake_ame/features/library/presentation/widgets/justified_gallery_layout.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("fills balanced photo rows to the available width", () {
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
    for (final row in rows) {
      final imageWidth = row.cells.fold<double>(
        0,
        (sum, cell) => sum + cell.width,
      );
      final totalWidth = imageWidth + spacing * (row.cells.length - 1);
      expect(row.isJustified, isTrue);
      expect(totalWidth, closeTo(availableWidth, 0.001));
    }
  });

  test("does not enlarge a sparse row beyond the configured limit", () {
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
