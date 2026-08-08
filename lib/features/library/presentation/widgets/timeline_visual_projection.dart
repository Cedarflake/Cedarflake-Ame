class TimelineVisualMappingPoint {
  const TimelineVisualMappingPoint({
    required this.logicalValue,
    required this.visualValue,
  });

  final double logicalValue;
  final double visualValue;
}

class TimelineVisualProjection {
  TimelineVisualProjection(Iterable<TimelineVisualMappingPoint> points)
    : _points = _normalize(points);

  final List<TimelineVisualMappingPoint> _points;

  double toVisual(double logicalValue) => _interpolate(
    logicalValue.clamp(0.0, 1.0).toDouble(),
    input: (point) => point.logicalValue,
    output: (point) => point.visualValue,
  );

  double toLogical(double visualValue) => _interpolate(
    visualValue.clamp(0.0, 1.0).toDouble(),
    input: (point) => point.visualValue,
    output: (point) => point.logicalValue,
  );

  double _interpolate(
    double value, {
    required double Function(TimelineVisualMappingPoint point) input,
    required double Function(TimelineVisualMappingPoint point) output,
  }) {
    for (var index = 1; index < _points.length; index++) {
      final lower = _points[index - 1];
      final upper = _points[index];
      final upperInput = input(upper);
      if (value > upperInput) {
        continue;
      }
      final lowerInput = input(lower);
      final span = upperInput - lowerInput;
      if (span <= 0) {
        return output(upper);
      }
      final fraction = ((value - lowerInput) / span).clamp(0.0, 1.0).toDouble();
      return output(lower) + ((output(upper) - output(lower)) * fraction);
    }
    return output(_points.last);
  }

  static List<TimelineVisualMappingPoint> _normalize(
    Iterable<TimelineVisualMappingPoint> points,
  ) {
    final sorted = [
      const TimelineVisualMappingPoint(logicalValue: 0, visualValue: 0),
      for (final point in points)
        if (point.logicalValue > 0 && point.logicalValue < 1) point,
      const TimelineVisualMappingPoint(logicalValue: 1, visualValue: 1),
    ]..sort((left, right) => left.logicalValue.compareTo(right.logicalValue));
    final deduplicated = <TimelineVisualMappingPoint>[];
    for (final point in sorted) {
      final normalized = TimelineVisualMappingPoint(
        logicalValue: point.logicalValue.clamp(0.0, 1.0).toDouble(),
        visualValue: point.visualValue.clamp(0.0, 1.0).toDouble(),
      );
      if (deduplicated.isNotEmpty &&
          (deduplicated.last.logicalValue - normalized.logicalValue).abs() <
              0.000001) {
        deduplicated[deduplicated.length - 1] = normalized;
      } else {
        deduplicated.add(normalized);
      }
    }
    final monotonic = <TimelineVisualMappingPoint>[];
    var previousVisual = 0.0;
    for (final point in deduplicated) {
      final visualValue = point.visualValue
          .clamp(previousVisual, 1.0)
          .toDouble();
      monotonic.add(
        TimelineVisualMappingPoint(
          logicalValue: point.logicalValue,
          visualValue: visualValue,
        ),
      );
      previousVisual = visualValue;
    }
    return List.unmodifiable(monotonic);
  }
}
