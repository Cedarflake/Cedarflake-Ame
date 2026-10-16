import "dart:math" as math;

import "package:flutter/material.dart";

abstract final class LibraryGalleryCardMetrics {
  static const minimumWidth = 128.0;
  static const overlayInset = 4.0;
  static const actionGap = 4.0;
  static const selectionExtent = 40.0;
  static const duplicateIconExtent = 18.0;
  static const duplicateHorizontalPadding = 8.0;
  static const maximumIconLabelGap = 8.0;

  static TextStyle badgeTextStyleOf(BuildContext context) =>
      TextButtonTheme.of(context).style?.textStyle?.resolve({}) ??
      Theme.of(context).textTheme.labelLarge ??
      DefaultTextStyle.of(context).style;

  static double minimumWidthOf(
    BuildContext context, {
    required int maximumLocationCount,
  }) {
    final digits = math.max(2, maximumLocationCount).toString().length;
    final style = badgeTextStyleOf(context);
    final painter = TextPainter(
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
      locale: Localizations.maybeLocaleOf(context),
      maxLines: 1,
    );
    var countWidth = 0.0;
    try {
      // Reserve the widest count of this length before asynchronous badges arrive.
      // The catalog's location bound keeps a single stable floor for the whole query.
      for (var digit = 0; digit <= 9; digit++) {
        painter.text = TextSpan(text: "$digit" * digits, style: style);
        painter.layout();
        countWidth = math.max(countWidth, painter.width);
      }
    } finally {
      painter.dispose();
    }
    final controlsWidth =
        overlayInset * 2 +
        duplicateHorizontalPadding * 2 +
        duplicateIconExtent +
        maximumIconLabelGap +
        countWidth +
        actionGap +
        selectionExtent;
    return math.max(minimumWidth, controlsWidth.ceilToDouble());
  }
}
