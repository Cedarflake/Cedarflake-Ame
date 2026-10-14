import "dart:math" as math;
import "dart:ui";

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

  double percentageForScale(double scale) => scale * 100;

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
