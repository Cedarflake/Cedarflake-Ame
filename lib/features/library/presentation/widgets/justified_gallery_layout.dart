import "dart:math" as math;

import "library_gallery_card_metrics.dart";

class JustifiedGalleryLayout {
  const JustifiedGalleryLayout({
    required this.targetRowHeight,
    required this.spacing,
    this.minimumCellWidth = LibraryGalleryCardMetrics.minimumWidth,
  });

  final double targetRowHeight;
  final double spacing;
  final double minimumCellWidth;

  List<JustifiedGalleryRow> compute({
    required List<double> aspectRatios,
    required double availableWidth,
  }) => computeRows(
    itemCount: aspectRatios.length,
    aspectRatioAt: (index) => aspectRatios[index],
    availableWidth: availableWidth,
  ).toList();

  Iterable<JustifiedGalleryRow> computeRows({
    required int itemCount,
    required double Function(int) aspectRatioAt,
    required double availableWidth,
  }) sync* {
    if (itemCount == 0 || availableWidth <= 0) return;

    final minimumAspectRatio =
        math.min(minimumCellWidth, availableWidth) / targetRowHeight;
    var ratios = <double>[];
    var rowStart = 0;
    var naturalRowWidth = 0.0;

    for (var itemIndex = 0; itemIndex < itemCount; itemIndex++) {
      final ratio = math.max(
        _normalizeAspectRatio(aspectRatioAt(itemIndex)),
        minimumAspectRatio,
      );
      final naturalWidth = targetRowHeight * ratio;
      final nextWidth = naturalRowWidth == 0
          ? naturalWidth
          : naturalRowWidth + spacing + naturalWidth;
      if (naturalRowWidth > 0 && nextWidth > availableWidth) {
        final minimumOccupiedWidth =
            minimumCellWidth * (ratios.length + 1) + spacing * ratios.length;
        final fitsBetter =
            nextWidth - availableWidth < availableWidth - naturalRowWidth;
        if (minimumOccupiedWidth <= availableWidth && fitsBetter) {
          ratios.add(ratio);
          yield _buildRow(
            ratios: ratios,
            start: rowStart,
            availableWidth: availableWidth,
            shouldFillWidth: true,
          );
          ratios = [];
          rowStart = itemIndex + 1;
          naturalRowWidth = 0;
          continue;
        }
        yield _buildRow(
          ratios: ratios,
          start: rowStart,
          availableWidth: availableWidth,
          shouldFillWidth: true,
        );
        ratios = [];
        rowStart = itemIndex;
        naturalRowWidth = 0;
      }
      ratios.add(ratio);
      naturalRowWidth += (ratios.length > 1 ? spacing : 0) + naturalWidth;
    }

    if (ratios.isNotEmpty) {
      yield _buildRow(
        ratios: ratios,
        start: rowStart,
        availableWidth: availableWidth,
        shouldFillWidth: false,
      );
    }
  }

  JustifiedGalleryRow _buildRow({
    required List<double> ratios,
    required int start,
    required double availableWidth,
    required bool shouldFillWidth,
  }) {
    final rowRatios = ratios;
    final availableImageWidth =
        availableWidth - spacing * (rowRatios.length - 1);
    final naturalImageWidth = rowRatios.fold<double>(
      0,
      (sum, ratio) => sum + (targetRowHeight * ratio),
    );
    final widthScale = shouldFillWidth && naturalImageWidth > 0
        ? availableImageWidth / naturalImageWidth
        : 1.0;
    final floor = math.min(minimumCellWidth, availableWidth);
    final flexibleWidth = naturalImageWidth - floor * rowRatios.length;
    final availableFlexibleWidth =
        availableImageWidth - floor * rowRatios.length;
    final cells = <JustifiedGalleryCell>[];
    var occupiedWidth = 0.0;

    for (var cellIndex = 0; cellIndex < rowRatios.length; cellIndex++) {
      final isLastCell = cellIndex == rowRatios.length - 1;
      final naturalWidth = targetRowHeight * rowRatios[cellIndex];
      final scaledWidth = widthScale < 1 && flexibleWidth > 0
          ? floor +
                (naturalWidth - floor) * availableFlexibleWidth / flexibleWidth
          : naturalWidth * widthScale;
      final width = shouldFillWidth && isLastCell
          ? availableImageWidth - occupiedWidth
          : scaledWidth.clamp(0.0, availableImageWidth);
      cells.add(
        JustifiedGalleryCell(itemIndex: start + cellIndex, width: width),
      );
      occupiedWidth += width;
    }

    return JustifiedGalleryRow(
      height: targetRowHeight,
      cells: cells,
      isJustified: shouldFillWidth,
    );
  }

  static double _normalizeAspectRatio(double value) {
    if (!value.isFinite || value <= 0) {
      return 1;
    }
    return value.clamp(0.2, 5.0).toDouble();
  }
}

class JustifiedGalleryRow {
  const JustifiedGalleryRow({
    required this.height,
    required this.cells,
    required this.isJustified,
  });

  final double height;
  final List<JustifiedGalleryCell> cells;
  final bool isJustified;
}

class JustifiedGalleryCell {
  const JustifiedGalleryCell({required this.itemIndex, required this.width});

  final int itemIndex;
  final double width;
}
