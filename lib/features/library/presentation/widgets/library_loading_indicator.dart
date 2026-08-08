import "dart:math" as math;

import "package:flutter/material.dart";

class LibraryLoadingIndicator extends StatelessWidget {
  const LibraryLoadingIndicator({
    this.maximumDimension = 32,
    this.minimumInset = 12,
    this.strokeWidth = 3,
    super.key,
  });

  final double maximumDimension;
  final double minimumInset;
  final double strokeWidth;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final availableWidth = constraints.hasBoundedWidth
            ? constraints.maxWidth
            : maximumDimension + minimumInset;
        final availableHeight = constraints.hasBoundedHeight
            ? constraints.maxHeight
            : maximumDimension + minimumInset;
        final dimension = math.min(
          maximumDimension,
          math.max(
            0.0,
            math.min(availableWidth, availableHeight) - minimumInset,
          ),
        );
        if (dimension < 8) {
          return const SizedBox.shrink();
        }
        return Center(
          child: SizedBox.square(
            dimension: dimension,
            child: CircularProgressIndicator(strokeWidth: strokeWidth),
          ),
        );
      },
    );
  }
}
