import "dart:math" as math;
import "dart:ui";

import "../../../settings/application/ame_preferences.dart";

class LibraryViewerZoomGeometry {
  const LibraryViewerZoomGeometry({
    required this.imageSize,
    required this.viewportSize,
  });

  static const _defaultMinimumScale = 0.25;
  static const _defaultMaximumScale = 8.0;

  final Size imageSize;
  final Size viewportSize;

  bool get hasImageDimensions => !imageSize.isEmpty && !viewportSize.isEmpty;

  double get fitImageScale {
    if (!hasImageDimensions) {
      return 1;
    }
    return math.min(
      viewportSize.width / imageSize.width,
      viewportSize.height / imageSize.height,
    );
  }

  double get actualSizeScale => 1 / fitImageScale;

  double get minimumScale {
    if (!hasImageDimensions) {
      return _defaultMinimumScale;
    }
    if (actualSizeScale < 0.01) {
      return actualSizeScale / 4;
    }
    return math
        .min(_defaultMinimumScale, actualSizeScale / 4)
        .clamp(0.01, _defaultMinimumScale);
  }

  double get maximumScale {
    if (!hasImageDimensions) {
      return _defaultMaximumScale;
    }
    return math
        .max(_defaultMaximumScale, actualSizeScale * 4)
        .clamp(_defaultMaximumScale, 1024);
  }

  double percentageForScale(double scale) => scale * fitImageScale * 100;

  FilterQuality filterQualityForScale(double scale, double devicePixelRatio) {
    if (!hasImageDimensions) return FilterQuality.medium;
    final physicalScale = scale * fitImageScale * devicePixelRatio;
    if ((physicalScale - 1).abs() < 0.000001) return FilterQuality.low;
    return physicalScale < 1 ? FilterQuality.medium : FilterQuality.high;
  }

  double openingScale(ImageViewerOpenBehavior behavior) {
    return switch (behavior) {
      ImageViewerOpenBehavior.automatic => math.min(1, actualSizeScale),
      ImageViewerOpenBehavior.fitWindow => 1,
      ImageViewerOpenBehavior.actualSize => actualSizeScale,
    };
  }

  bool isActualSize(double scale) => (scale / actualSizeScale - 1).abs() < 0.01;

  double sliderValueForScale(double scale) {
    final bounded = scale.clamp(minimumScale, maximumScale).toDouble();
    return (math.log(bounded) - math.log(minimumScale)) /
        (math.log(maximumScale) - math.log(minimumScale));
  }

  double scaleForSliderValue(double value) {
    return math.exp(
      math.log(minimumScale) +
          value * (math.log(maximumScale) - math.log(minimumScale)),
    );
  }
}
