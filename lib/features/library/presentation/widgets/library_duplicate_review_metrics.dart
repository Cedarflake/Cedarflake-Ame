import "dart:math" as math;

import "package:flutter/material.dart";

import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";

/// Measures the shared content/footer slots without coupling them to column count.
class LibraryDuplicateReviewMetrics {
  const LibraryDuplicateReviewMetrics._();
  static const padding = 12.0;
  static const gap = 8.0;
  static const thumbnailExtent = 80.0;
  static const iconActionExtent = 48.0;
  static final iconActionStyle = IconButton.styleFrom(
    fixedSize: const Size.square(iconActionExtent),
    visualDensity: VisualDensity.standard,
    iconSize: 20,
  );
  static final actionStyle = TextButton.styleFrom(
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
    minimumSize: const Size(64, 48),
    iconSize: 18,
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  );

  static double height(
    BuildContext context,
    double width, {
    required bool member,
  }) {
    final text = Theme.of(context).textTheme;
    final body = _measure(context, "图片", text.bodyMedium);
    final title = _measure(context, "图片", text.titleMedium);
    var detailHeight = body.height * 2;
    if (member) {
      final textWidth = math.max(
        1.0,
        width - padding * 2 - thumbnailExtent - 24 - iconActionExtent * 2,
      );
      var statusHeight = 0.0;
      for (final value in LibraryDuplicateAvailability.values) {
        final label = LibraryDuplicateStrings.availability(value);
        if (label == null) continue;
        statusHeight = math.max(
          statusHeight,
          _measure(context, label, text.bodyMedium, maxWidth: textWidth).height,
        );
      }
      detailHeight += statusHeight;
    }
    final content = math.max(thumbnailExtent, title.height + 4 + detailHeight);
    if (member) return padding * 2 + content;
    final available = width - padding * 2;
    final label = _measure(
      context,
      LibraryDuplicateStrings.viewLocations,
      text.labelLarge,
      maxWidth: math.max(1, available - 24),
    );
    return padding * 2 + content + gap + math.max(48.0, label.height + 16);
  }

  static Size _measure(
    BuildContext context,
    String value,
    TextStyle? style, {
    double maxWidth = double.infinity,
  }) {
    final painter = TextPainter(
      text: TextSpan(text: value, style: style),
      textScaler: MediaQuery.textScalerOf(context),
      textDirection: Directionality.of(context),
    )..layout(maxWidth: maxWidth);
    final size = painter.size;
    painter.dispose();
    return size;
  }
}
