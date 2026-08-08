class JustifiedGalleryLayout {
  const JustifiedGalleryLayout({
    required this.targetRowHeight,
    required this.spacing,
  });

  final double targetRowHeight;
  final double spacing;

  List<JustifiedGalleryRow> compute({
    required List<double> aspectRatios,
    required double availableWidth,
  }) {
    if (aspectRatios.isEmpty || availableWidth <= 0) {
      return const [];
    }

    final ratios = aspectRatios.map(_normalizeAspectRatio).toList();
    final rows = <JustifiedGalleryRow>[];
    var rowStart = 0;
    var naturalRowWidth = 0.0;

    for (var itemIndex = 0; itemIndex < ratios.length; itemIndex++) {
      final naturalWidth = targetRowHeight * ratios[itemIndex];
      final nextWidth = naturalRowWidth == 0
          ? naturalWidth
          : naturalRowWidth + spacing + naturalWidth;
      if (naturalRowWidth > 0 && nextWidth > availableWidth) {
        rows.add(
          _buildRow(
            ratios: ratios,
            start: rowStart,
            end: itemIndex,
            availableWidth: availableWidth,
            shouldFillWidth: true,
          ),
        );
        rowStart = itemIndex;
        naturalRowWidth = naturalWidth;
      } else {
        naturalRowWidth = nextWidth;
      }
    }

    rows.add(
      _buildRow(
        ratios: ratios,
        start: rowStart,
        end: ratios.length,
        availableWidth: availableWidth,
        shouldFillWidth: false,
      ),
    );
    return rows;
  }

  JustifiedGalleryRow _buildRow({
    required List<double> ratios,
    required int start,
    required int end,
    required double availableWidth,
    required bool shouldFillWidth,
  }) {
    final rowRatios = ratios.sublist(start, end);
    final availableImageWidth =
        availableWidth - spacing * (rowRatios.length - 1);
    final naturalImageWidth = rowRatios.fold<double>(
      0,
      (sum, ratio) => sum + (targetRowHeight * ratio),
    );
    final widthScale = shouldFillWidth && naturalImageWidth > 0
        ? availableImageWidth / naturalImageWidth
        : 1.0;
    final cells = <JustifiedGalleryCell>[];
    var occupiedWidth = 0.0;

    for (var cellIndex = 0; cellIndex < rowRatios.length; cellIndex++) {
      final isLastCell = cellIndex == rowRatios.length - 1;
      final naturalWidth = targetRowHeight * rowRatios[cellIndex];
      final width = shouldFillWidth && isLastCell
          ? availableImageWidth - occupiedWidth
          : (naturalWidth * widthScale).clamp(0.0, availableImageWidth);
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
