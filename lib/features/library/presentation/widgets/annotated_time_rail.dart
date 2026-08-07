import "package:flutter/material.dart";

class TimelineRailBucket {
  const TimelineRailBucket({
    required this.id,
    required this.label,
    required this.contentExtent,
    required this.year,
    this.isUnknown = false,
  });

  final String id;
  final String label;
  final double contentExtent;
  final int? year;
  final bool isUnknown;
}

class AnnotatedTimeRail extends StatelessWidget {
  const AnnotatedTimeRail({
    required this.value,
    required this.buckets,
    required this.onChanged,
    this.onChangeEnd,
    this.onBucketActivated,
    super.key,
  });

  static const double _controlWidth = kMinInteractiveDimension;
  static const double _endpointInset = 12;
  static const double _markerHeight = 24;
  static const double _markerWidth = 24;
  static const double _railWidth = 80;
  static const double _axisX = _railWidth - (_controlWidth / 2);

  final double value;
  final List<TimelineRailBucket> buckets;
  final ValueChanged<double> onChanged;
  final ValueChanged<double>? onChangeEnd;
  final ValueChanged<TimelineRailBucket>? onBucketActivated;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final anchors = _buildAnchors(buckets);
    if (anchors.isEmpty) {
      return const SizedBox.shrink();
    }
    return SizedBox(
      width: _railWidth,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(left: BorderSide(color: colorScheme.outlineVariant)),
        ),
        child: Column(
          children: [
            Align(
              alignment: Alignment.centerRight,
              child: SizedBox(
                width: _controlWidth,
                height: kMinInteractiveDimension,
                child: IconButton(
                  key: const Key("timeline-previous"),
                  tooltip: "向较新的日期移动",
                  onPressed: () => _step(anchors, -1),
                  icon: const Icon(Icons.arrow_drop_up),
                ),
              ),
            ),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final visibleYears = _visibleYearIds(
                    anchors,
                    constraints.maxHeight,
                  );
                  final currentBucketId = _nearestBucket(
                    anchors,
                    value,
                  ).bucket.id;
                  return Stack(
                    clipBehavior: Clip.none,
                    children: [
                      Positioned(
                        top: 0,
                        right: 0,
                        bottom: 0,
                        width: _controlWidth,
                        child: _VerticalMaterialSlider(
                          value: value,
                          endpointInset: _endpointInset,
                          onChanged: onChanged,
                          onChangeEnd: _commit,
                          semanticLabelFor: (sliderValue) =>
                              _nearestBucket(anchors, sliderValue).bucket.label,
                        ),
                      ),
                      for (final anchor in anchors)
                        ..._buildMarkers(
                          context,
                          anchor: anchor,
                          railHeight: constraints.maxHeight,
                          isYearVisible: visibleYears.contains(
                            anchor.bucket.id,
                          ),
                          isCurrent: currentBucketId == anchor.bucket.id,
                        ),
                    ],
                  );
                },
              ),
            ),
            Align(
              alignment: Alignment.centerRight,
              child: SizedBox(
                width: _controlWidth,
                height: kMinInteractiveDimension,
                child: IconButton(
                  key: const Key("timeline-next"),
                  tooltip: "向较早的日期移动",
                  onPressed: () => _step(anchors, 1),
                  icon: const Icon(Icons.arrow_drop_down),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildMarkers(
    BuildContext context, {
    required _TimelineAnchor anchor,
    required double railHeight,
    required bool isYearVisible,
    required bool isCurrent,
  }) {
    final usableHeight = (railHeight - (_endpointInset * 2)).clamp(
      0.0,
      double.infinity,
    );
    final top =
        _endpointInset + (anchor.value * usableHeight) - (_markerHeight / 2);
    return [
      Positioned(
        key: ValueKey("time-marker-${anchor.bucket.id}"),
        left: _axisX - (_markerWidth / 2),
        top: top,
        width: _markerWidth,
        height: _markerHeight,
        child: Tooltip(
          message: anchor.bucket.label,
          child: Material(
            color: Colors.transparent,
            child: InkResponse(
              onTap: () => _activate(anchor.value),
              radius: 18,
              child: Center(
                child: Container(
                  width: isCurrent ? 6 : 4,
                  height: isCurrent ? 6 : 4,
                  decoration: BoxDecoration(
                    color: isCurrent
                        ? Theme.of(context).colorScheme.onPrimary
                        : Theme.of(context).colorScheme.outline,
                    shape: BoxShape.circle,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
      if (isYearVisible)
        Positioned(
          key: ValueKey("time-label-${anchor.bucket.id}"),
          left: 2,
          top: top,
          width: 40,
          height: _markerHeight,
          child: Tooltip(
            message: anchor.bucket.label,
            child: Material(
              color: Colors.transparent,
              child: InkResponse(
                onTap: () => _activate(anchor.value),
                radius: 18,
                child: Center(
                  child: Text(
                    anchor.bucket.isUnknown ? "未知" : "${anchor.bucket.year}",
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: isCurrent
                          ? Theme.of(context).colorScheme.primary
                          : null,
                      fontWeight: isCurrent ? FontWeight.w700 : null,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
    ];
  }

  void _step(List<_TimelineAnchor> anchors, int direction) {
    final current = _nearestBucketIndex(anchors, value);
    final target = (current + direction).clamp(0, anchors.length - 1);
    _activate(anchors[target].value);
  }

  void _activate(double nextValue) {
    onChanged(nextValue);
    _commit(nextValue);
  }

  void _commit(double nextValue) {
    onChangeEnd?.call(nextValue);
    if (buckets.isNotEmpty) {
      onBucketActivated?.call(
        _nearestBucket(_buildAnchors(buckets), nextValue).bucket,
      );
    }
  }

  static List<_TimelineAnchor> _buildAnchors(List<TimelineRailBucket> buckets) {
    if (buckets.isEmpty) {
      return const [];
    }
    final maximumStart = buckets
        .take(buckets.length - 1)
        .fold<double>(0, (sum, bucket) => sum + bucket.contentExtent);
    final anchors = <_TimelineAnchor>[];
    var runningExtent = 0.0;
    int? previousYear;
    for (final bucket in buckets) {
      anchors.add(
        _TimelineAnchor(
          bucket: bucket,
          value: maximumStart <= 0
              ? 0.0
              : (runningExtent / maximumStart).clamp(0.0, 1.0).toDouble(),
          isYearStart: bucket.year != previousYear,
        ),
      );
      runningExtent += bucket.contentExtent;
      previousYear = bucket.year;
    }
    return anchors;
  }

  static Set<String> _visibleYearIds(
    List<_TimelineAnchor> anchors,
    double railHeight,
  ) {
    final visible = <String>{};
    var lastBottom = double.negativeInfinity;
    for (final anchor in anchors) {
      if (!anchor.isYearStart && !anchor.bucket.isUnknown) {
        continue;
      }
      final usableHeight = (railHeight - (_endpointInset * 2)).clamp(
        0.0,
        double.infinity,
      );
      final top =
          _endpointInset + (anchor.value * usableHeight) - (_markerHeight / 2);
      if (visible.isEmpty || top >= lastBottom + 4) {
        visible.add(anchor.bucket.id);
        lastBottom = top + _markerHeight;
      }
    }
    return visible;
  }

  static _TimelineAnchor _nearestBucket(
    List<_TimelineAnchor> anchors,
    double value,
  ) {
    return anchors[_nearestBucketIndex(anchors, value)];
  }

  static int _nearestBucketIndex(List<_TimelineAnchor> anchors, double value) {
    var nearestIndex = 0;
    var nearestDistance = (anchors.first.value - value).abs();
    for (var index = 1; index < anchors.length; index++) {
      final distance = (anchors[index].value - value).abs();
      if (distance < nearestDistance) {
        nearestIndex = index;
        nearestDistance = distance;
      }
    }
    return nearestIndex;
  }
}

double timelineRailValueForBucket(
  List<TimelineRailBucket> buckets,
  String bucketId,
) {
  if (buckets.isEmpty) {
    return 0;
  }
  final maximumStart = buckets
      .take(buckets.length - 1)
      .fold<double>(0, (sum, bucket) => sum + bucket.contentExtent);
  if (maximumStart <= 0) {
    return 0;
  }
  var runningExtent = 0.0;
  for (final bucket in buckets) {
    if (bucket.id == bucketId) {
      return (runningExtent / maximumStart).clamp(0.0, 1.0).toDouble();
    }
    runningExtent += bucket.contentExtent;
  }
  return 0;
}

class _VerticalMaterialSlider extends StatefulWidget {
  const _VerticalMaterialSlider({
    required this.value,
    required this.endpointInset,
    required this.onChanged,
    required this.onChangeEnd,
    required this.semanticLabelFor,
  });

  final double value;
  final double endpointInset;
  final ValueChanged<double> onChanged;
  final ValueChanged<double> onChangeEnd;
  final String Function(double value) semanticLabelFor;

  @override
  State<_VerticalMaterialSlider> createState() =>
      _VerticalMaterialSliderState();
}

class _VerticalMaterialSliderState extends State<_VerticalMaterialSlider> {
  final FocusNode _focusNode = FocusNode(
    debugLabel: "Annotated timeline Material Slider",
  );

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final sliderTheme = SliderTheme.of(context).copyWith(
      trackHeight: 16,
      activeTrackColor: Colors.transparent,
      inactiveTrackColor: Colors.transparent,
      thumbColor: colorScheme.primary,
      trackShape: const RoundedRectSliderTrackShape(),
      thumbShape: const HandleThumbShape(),
      thumbSize: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.focused) ||
            states.contains(WidgetState.pressed)) {
          return const Size(2, 28);
        }
        return const Size(4, 28);
      }),
    );
    return Stack(
      fit: StackFit.expand,
      children: [
        Center(
          child: SizedBox(
            key: const Key("timeline-track-background"),
            width: 16,
            height: double.infinity,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.secondaryContainer,
                borderRadius: BorderRadius.circular(8),
              ),
            ),
          ),
        ),
        SliderTheme(
          data: sliderTheme,
          child: RotatedBox(
            quarterTurns: 3,
            child: Slider(
              key: const Key("timeline-slider"),
              value: 1 - widget.value,
              onChanged: (value) => widget.onChanged(1 - value),
              onChangeEnd: (value) => widget.onChangeEnd(1 - value),
              allowedInteraction: SliderInteraction.tapAndSlide,
              focusNode: _focusNode,
              padding: EdgeInsets.symmetric(horizontal: widget.endpointInset),
              semanticFormatterCallback: (value) =>
                  widget.semanticLabelFor(1 - value),
            ),
          ),
        ),
      ],
    );
  }
}

class _TimelineAnchor {
  const _TimelineAnchor({
    required this.bucket,
    required this.value,
    required this.isYearStart,
  });

  final TimelineRailBucket bucket;
  final double value;
  final bool isYearStart;
}
