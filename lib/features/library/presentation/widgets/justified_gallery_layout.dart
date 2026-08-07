class JustifiedGalleryLayout {
  const JustifiedGalleryLayout({
    required this.targetRowHeight,
    required this.spacing,
    this.maximumRowHeightFactor = 1.5,
  });

  final double targetRowHeight;
  final double spacing;
  final double maximumRowHeightFactor;

  List<JustifiedGalleryRow> compute({
    required List<double> aspectRatios,
    required double availableWidth,
  }) {
    if (aspectRatios.isEmpty || availableWidth <= 0) {
      return const [];
    }

    final ratios = aspectRatios.map(_normalizeAspectRatio).toList();
    final totalAspectRatio = ratios.fold<double>(
      0,
      (sum, ratio) => sum + ratio,
    );
    final targetAspectRatio = availableWidth / targetRowHeight;
    final rowCount = (totalAspectRatio / targetAspectRatio)
        .round()
        .clamp(1, ratios.length)
        .toInt();
    final rows = <JustifiedGalleryRow>[];
    var itemIndex = 0;
    var remainingAspectRatio = totalAspectRatio;

    for (var rowIndex = 0; rowIndex < rowCount; rowIndex++) {
      final remainingRows = rowCount - rowIndex;
      final rowStart = itemIndex;
      var rowAspectRatio = 0.0;

      if (remainingRows == 1) {
        while (itemIndex < ratios.length) {
          rowAspectRatio += ratios[itemIndex];
          itemIndex++;
        }
      } else {
        final idealAspectRatio = remainingAspectRatio / remainingRows;
        final lastAllowedIndex = ratios.length - (remainingRows - 1);
        while (itemIndex < lastAllowedIndex) {
          final nextAspectRatio = rowAspectRatio + ratios[itemIndex];
          if (rowAspectRatio > 0 &&
              (rowAspectRatio - idealAspectRatio).abs() <
                  (nextAspectRatio - idealAspectRatio).abs()) {
            break;
          }
          rowAspectRatio = nextAspectRatio;
          itemIndex++;
        }
      }

      final rowRatios = ratios.sublist(rowStart, itemIndex);
      final imageWidth = availableWidth - spacing * (rowRatios.length - 1);
      final justifiedHeight = imageWidth / rowAspectRatio;
      final maximumRowHeight = targetRowHeight * maximumRowHeightFactor;
      final isJustified = justifiedHeight <= maximumRowHeight;
      final rowHeight = isJustified
          ? justifiedHeight
          : justifiedHeight.clamp(0.0, targetRowHeight).toDouble();
      final cells = <JustifiedGalleryCell>[];
      var occupiedWidth = 0.0;

      for (var cellIndex = 0; cellIndex < rowRatios.length; cellIndex++) {
        final isLastCell = cellIndex == rowRatios.length - 1;
        final width = isLastCell && isJustified
            ? imageWidth - occupiedWidth
            : rowHeight * rowRatios[cellIndex];
        cells.add(
          JustifiedGalleryCell(itemIndex: rowStart + cellIndex, width: width),
        );
        occupiedWidth += width;
      }

      rows.add(
        JustifiedGalleryRow(
          height: rowHeight,
          cells: cells,
          isJustified: isJustified,
        ),
      );
      remainingAspectRatio -= rowAspectRatio;
    }

    return rows;
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
