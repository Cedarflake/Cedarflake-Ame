import "dart:ui";

class AmeWindowPlacement {
  const AmeWindowPlacement({
    required this.left,
    required this.top,
    required this.width,
    required this.height,
    required this.isMaximized,
  });

  final double left;
  final double top;
  final double width;
  final double height;
  final bool isMaximized;

  Rect get bounds => Rect.fromLTWH(left, top, width, height);

  AmeWindowPlacement copyWith({
    double? left,
    double? top,
    double? width,
    double? height,
    bool? isMaximized,
  }) {
    return AmeWindowPlacement(
      left: left ?? this.left,
      top: top ?? this.top,
      width: width ?? this.width,
      height: height ?? this.height,
      isMaximized: isMaximized ?? this.isMaximized,
    );
  }
}

abstract interface class AmeWindowPreferenceStore {
  Future<AmeWindowPlacement?> loadWindowPlacement();

  Future<void> saveWindowPlacement(AmeWindowPlacement placement);
}

AmeWindowPlacement? normalizeAmeWindowPlacement(
  AmeWindowPlacement? placement, {
  required List<Rect> visibleScreenBounds,
  required Size minimumSize,
}) {
  if (placement == null ||
      visibleScreenBounds.isEmpty ||
      !_isFinitePlacement(placement)) {
    return null;
  }

  final savedBounds = placement.bounds;
  var targetScreen = visibleScreenBounds.first;
  var largestIntersection = -1.0;
  for (final screen in visibleScreenBounds) {
    final intersection = savedBounds.intersect(screen);
    final area = intersection.isEmpty
        ? 0.0
        : intersection.width * intersection.height;
    if (area > largestIntersection) {
      targetScreen = screen;
      largestIntersection = area;
    }
  }

  final width = _constrainDimension(
    placement.width,
    minimumSize.width,
    targetScreen.width,
  );
  final height = _constrainDimension(
    placement.height,
    minimumSize.height,
    targetScreen.height,
  );
  final maxLeft = targetScreen.right - width;
  final maxTop = targetScreen.bottom - height;
  final left = placement.left.clamp(
    targetScreen.left,
    maxLeft < targetScreen.left ? targetScreen.left : maxLeft,
  );
  final top = placement.top.clamp(
    targetScreen.top,
    maxTop < targetScreen.top ? targetScreen.top : maxTop,
  );

  return AmeWindowPlacement(
    left: left.toDouble(),
    top: top.toDouble(),
    width: width,
    height: height,
    isMaximized: placement.isMaximized,
  );
}

bool _isFinitePlacement(AmeWindowPlacement placement) {
  return placement.left.isFinite &&
      placement.top.isFinite &&
      placement.width.isFinite &&
      placement.height.isFinite &&
      placement.width > 0 &&
      placement.height > 0;
}

double _constrainDimension(double value, double minimum, double maximum) {
  final effectiveMinimum = minimum > maximum ? maximum : minimum;
  return value.clamp(effectiveMinimum, maximum).toDouble();
}
