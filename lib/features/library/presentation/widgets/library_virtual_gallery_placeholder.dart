import "dart:math" as math;

import "package:flutter/material.dart";

class LibraryVirtualGalleryPlaceholder extends StatelessWidget {
  const LibraryVirtualGalleryPlaceholder({
    required this.extent,
    required this.horizontalPadding,
    required this.targetTileExtent,
    super.key,
  });

  final double extent;
  final double horizontalPadding;
  final double targetTileExtent;

  @override
  Widget build(BuildContext context) {
    if (extent <= 0) {
      return const SizedBox.shrink();
    }
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      height: extent,
      width: double.infinity,
      child: RepaintBoundary(
        child: CustomPaint(
          painter: _VirtualGalleryPlaceholderPainter(
            horizontalPadding: horizontalPadding,
            targetTileExtent: targetTileExtent,
            fillColor: colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.72,
            ),
          ),
        ),
      ),
    );
  }
}

class _VirtualGalleryPlaceholderPainter extends CustomPainter {
  const _VirtualGalleryPlaceholderPainter({
    required this.horizontalPadding,
    required this.targetTileExtent,
    required this.fillColor,
  });

  static const double _spacing = 6;

  final double horizontalPadding;
  final double targetTileExtent;
  final Color fillColor;

  @override
  void paint(Canvas canvas, Size size) {
    final availableWidth = math.max(0.0, size.width - horizontalPadding);
    if (availableWidth <= 0 || size.height <= 0) {
      return;
    }
    final columnCount =
        ((availableWidth + _spacing) / (targetTileExtent + _spacing))
            .floor()
            .clamp(1, 1000);
    final tileExtent =
        (availableWidth - (_spacing * (columnCount - 1))) / columnCount;
    final rowStride = tileExtent + _spacing;
    final clip = canvas.getLocalClipBounds();
    final firstRow = math.max(0, (clip.top / rowStride).floor() - 1);
    final lastRow = math.min(
      (size.height / rowStride).ceil(),
      (clip.bottom / rowStride).ceil() + 1,
    );
    final paint = Paint()..color = fillColor;
    final radius = Radius.circular(math.min(10, tileExtent * 0.08));
    for (var row = firstRow; row < lastRow; row++) {
      final top = row * rowStride;
      for (var column = 0; column < columnCount; column++) {
        final left = horizontalPadding / 2 + column * rowStride;
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromLTWH(left, top, tileExtent, tileExtent),
            radius,
          ),
          paint,
        );
      }
    }
  }

  @override
  bool shouldRepaint(_VirtualGalleryPlaceholderPainter oldDelegate) {
    return horizontalPadding != oldDelegate.horizontalPadding ||
        targetTileExtent != oldDelegate.targetTileExtent ||
        fillColor != oldDelegate.fillColor;
  }
}
