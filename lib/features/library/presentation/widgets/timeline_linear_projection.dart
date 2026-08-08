class TimelineLinearProjection {
  TimelineLinearProjection({required double maximumOffset})
    : maximumOffset = maximumOffset.isFinite && maximumOffset > 0
          ? maximumOffset
          : 0;

  final double maximumOffset;

  double offsetToValue(double offset) {
    if (maximumOffset <= 0) {
      return 0;
    }
    return (offset / maximumOffset).clamp(0.0, 1.0).toDouble();
  }

  double valueToOffset(double value) {
    return value.clamp(0.0, 1.0).toDouble() * maximumOffset;
  }
}
