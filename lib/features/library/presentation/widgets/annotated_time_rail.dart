import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "timeline_annotation_visibility.dart";
import "timeline_linear_projection.dart";
import "timeline_rail_model.dart";
import "timeline_visual_projection.dart";
import "vertical_material_timeline_slider.dart";

export "timeline_rail_model.dart"
    show TimelineRailBucket, timelineRailValueForBucket;

class AnnotatedTimeRail extends StatefulWidget {
  const AnnotatedTimeRail({
    required this.value,
    required this.buckets,
    required this.onChanged,
    this.maximumScrollOffset,
    this.projection,
    this.onChangeStart,
    this.onChangeEnd,
    this.onStep,
    super.key,
  });

  final double value;
  final List<TimelineRailBucket> buckets;
  final double? maximumScrollOffset;
  final TimelineLinearProjection? projection;
  final ValueChanged<double> onChanged;
  final ValueChanged<double>? onChangeStart;
  final ValueChanged<double>? onChangeEnd;
  final ValueChanged<int>? onStep;

  static const double width = 80;

  @override
  State<AnnotatedTimeRail> createState() => _AnnotatedTimeRailState();
}

class _AnnotatedTimeRailState extends State<AnnotatedTimeRail> {
  static const double _controlWidth = kMinInteractiveDimension;
  static const double _endpointInset = 12;
  static const double _markerExtent = 4;
  static const double _markerMinimumGap = 6;
  static const double _labelExtent = 24;
  static const double _labelMinimumGap = 16;
  static const double _positionLineHeight = 3;
  static const double _positionLineWidth = 30;
  static const double _axisX = AnnotatedTimeRail.width - (_controlWidth / 2);

  double? _hoverValue;
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final projection =
        widget.projection ??
        TimelineLinearProjection(
          maximumOffset:
              widget.maximumScrollOffset ??
              inferredTimelineMaximumOffset(widget.buckets),
        );
    final anchors = buildTimelineRailAnchors(widget.buckets, projection);
    if (anchors.isEmpty) {
      return const SizedBox(width: AnnotatedTimeRail.width);
    }
    return SizedBox(
      width: AnnotatedTimeRail.width,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(left: BorderSide(color: colorScheme.outlineVariant)),
        ),
        child: Column(
          children: [
            _buildStepButton(
              key: const Key("timeline-previous"),
              tooltip: "向较新的内容移动一行",
              icon: Symbols.arrow_drop_up_rounded,
              direction: -1,
            ),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final labelPlacements = layoutTimelineAnnotations(
                    [
                      for (final anchor in anchors)
                        if (anchor.isYearStart || anchor.bucket.isUnknown)
                          TimelineAnnotationCandidate(
                            id: anchor.bucket.id,
                            value: anchor.value,
                            extent: _labelExtent,
                          ),
                    ],
                    railExtent: constraints.maxHeight,
                    startInset: _endpointInset,
                    endInset: _endpointInset,
                    minimumGap: _labelMinimumGap,
                  );
                  final labelCenters = {
                    for (final placement in labelPlacements)
                      placement.id: placement.center,
                  };
                  final usableHeight =
                      constraints.maxHeight - (_endpointInset * 2);
                  final visualProjection = TimelineVisualProjection([
                    if (usableHeight > 0)
                      for (final anchor in anchors)
                        if (labelCenters[anchor.bucket.id]
                            case final labelCenter?)
                          TimelineVisualMappingPoint(
                            logicalValue: anchor.value,
                            visualValue:
                                ((labelCenter - _endpointInset) / usableHeight)
                                    .clamp(0.0, 1.0)
                                    .toDouble(),
                          ),
                  ]);
                  final unlabeledMarkerIds = visibleTimelineAnnotationIds(
                    [
                      for (final anchor in anchors)
                        if (!labelCenters.containsKey(anchor.bucket.id))
                          TimelineAnnotationCandidate(
                            id: anchor.bucket.id,
                            value: visualProjection.toVisual(anchor.value),
                            extent: _markerExtent,
                          ),
                    ],
                    railExtent: constraints.maxHeight,
                    startInset: _endpointInset,
                    endInset: _endpointInset,
                    minimumGap: _markerMinimumGap,
                  );
                  final currentBucketId = timelineRailBucketAtValue(
                    anchors,
                    widget.value,
                  ).bucket.id;
                  return Listener(
                    key: const Key("timeline-hover-region"),
                    behavior: HitTestBehavior.translucent,
                    onPointerHover: (event) => _updateHover(
                      event.localPosition,
                      constraints.maxHeight,
                    ),
                    child: MouseRegion(
                      cursor: SystemMouseCursors.click,
                      onExit: (_) => setState(() => _hoverValue = null),
                      child: Stack(
                        clipBehavior: Clip.none,
                        children: [
                          Positioned(
                            top: 0,
                            right: 0,
                            bottom: 0,
                            width: _controlWidth,
                            child: VerticalMaterialTimelineSlider(
                              value: visualProjection.toVisual(widget.value),
                              endpointInset: _endpointInset,
                              onChangeStart: (visualValue) => _startInteraction(
                                visualProjection.toLogical(visualValue),
                              ),
                              onChanged: (visualValue) => widget.onChanged(
                                visualProjection.toLogical(visualValue),
                              ),
                              onChangeEnd: (visualValue) => _commit(
                                visualProjection.toLogical(visualValue),
                              ),
                              semanticLabelFor: (sliderValue) =>
                                  timelineRailBucketAtValue(
                                    anchors,
                                    visualProjection.toLogical(sliderValue),
                                  ).bucket.label,
                            ),
                          ),
                          _buildPositionLine(
                            key: const Key("timeline-current-line"),
                            value: visualProjection.toVisual(widget.value),
                            railHeight: constraints.maxHeight,
                            color: colorScheme.primary,
                          ),
                          for (final anchor in anchors)
                            ..._buildAnnotation(
                              context,
                              anchor: anchor,
                              railHeight: constraints.maxHeight,
                              visualValue: visualProjection.toVisual(
                                anchor.value,
                              ),
                              hasLabel: labelCenters.containsKey(
                                anchor.bucket.id,
                              ),
                              hasMarker:
                                  labelCenters.containsKey(anchor.bucket.id) ||
                                  _isUnlabeledMarkerVisible(
                                    anchor: anchor,
                                    railHeight: constraints.maxHeight,
                                    visualProjection: visualProjection,
                                    visibleMarkerIds: unlabeledMarkerIds,
                                    labelCenters: labelCenters.values,
                                  ),
                              isCurrent: currentBucketId == anchor.bucket.id,
                            ),
                          if (_isDragging)
                            _buildPositionLabel(
                              context,
                              key: const Key("timeline-drag-label"),
                              anchors: anchors,
                              visualProjection: visualProjection,
                              visualValue: visualProjection.toVisual(
                                widget.value,
                              ),
                              railHeight: constraints.maxHeight,
                            )
                          else if (_hoverValue case final hoverValue?)
                            ..._buildHoverPreview(
                              context,
                              anchors: anchors,
                              visualProjection: visualProjection,
                              visualValue: hoverValue,
                              railHeight: constraints.maxHeight,
                            ),
                        ],
                      ),
                    ),
                  );
                },
              ),
            ),
            _buildStepButton(
              key: const Key("timeline-next"),
              tooltip: "向较早的内容移动一行",
              icon: Symbols.arrow_drop_down_rounded,
              direction: 1,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStepButton({
    required Key key,
    required String tooltip,
    required IconData icon,
    required int direction,
  }) {
    return Align(
      alignment: Alignment.centerRight,
      child: SizedBox(
        width: _controlWidth,
        height: kMinInteractiveDimension,
        child: IconButton(
          key: key,
          tooltip: tooltip,
          onPressed: () => _step(direction),
          icon: Icon(icon),
        ),
      ),
    );
  }

  List<Widget> _buildAnnotation(
    BuildContext context, {
    required TimelineRailAnchor anchor,
    required double railHeight,
    required double visualValue,
    required bool hasLabel,
    required bool hasMarker,
    required bool isCurrent,
  }) {
    final markerCenter = _topForValue(visualValue, railHeight);
    final marker = Positioned(
      key: ValueKey("time-marker-${anchor.bucket.id}"),
      left: _axisX - (_markerExtent / 2),
      top: markerCenter - (_markerExtent / 2),
      width: _markerExtent,
      height: _markerExtent,
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.outline,
            shape: BoxShape.circle,
          ),
        ),
      ),
    );
    if (!hasLabel) {
      if (!hasMarker) {
        return const [];
      }
      return [marker];
    }
    return [
      if (hasMarker) marker,
      Positioned(
        key: ValueKey("time-label-${anchor.bucket.id}"),
        left: 2,
        top: markerCenter - (_labelExtent / 2),
        width: 40,
        height: _labelExtent,
        child: Tooltip(
          message: anchor.bucket.label,
          child: Align(
            alignment: Alignment.centerRight,
            child: Text(
              anchor.bucket.isUnknown ? "未知" : "${anchor.bucket.year}",
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: isCurrent ? Theme.of(context).colorScheme.primary : null,
                fontWeight: isCurrent ? FontWeight.w700 : null,
              ),
            ),
          ),
        ),
      ),
    ];
  }

  Positioned _buildPositionLine({
    required Key key,
    required double value,
    required double railHeight,
    required Color color,
  }) {
    return Positioned(
      key: key,
      left: _axisX - (_positionLineWidth / 2),
      top: _topForValue(value, railHeight) - (_positionLineHeight / 2),
      width: _positionLineWidth,
      height: _positionLineHeight,
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(_positionLineHeight / 2),
          ),
        ),
      ),
    );
  }

  List<Widget> _buildHoverPreview(
    BuildContext context, {
    required List<TimelineRailAnchor> anchors,
    required TimelineVisualProjection visualProjection,
    required double visualValue,
    required double railHeight,
  }) {
    final colorScheme = Theme.of(context).colorScheme;
    return [
      _buildPositionLine(
        key: const Key("timeline-hover-line"),
        value: visualValue,
        railHeight: railHeight,
        color: colorScheme.onSurfaceVariant.withValues(alpha: 0.45),
      ),
      _buildPositionLabel(
        context,
        key: const Key("timeline-hover-label"),
        anchors: anchors,
        visualProjection: visualProjection,
        visualValue: visualValue,
        railHeight: railHeight,
      ),
    ];
  }

  Positioned _buildPositionLabel(
    BuildContext context, {
    required Key key,
    required List<TimelineRailAnchor> anchors,
    required TimelineVisualProjection visualProjection,
    required double visualValue,
    required double railHeight,
  }) {
    final colorScheme = Theme.of(context).colorScheme;
    final lineTop = _topForValue(visualValue, railHeight);
    final labelTop = (lineTop - 14).clamp(0.0, railHeight - 28).toDouble();
    final label = timelineRailBucketAtValue(
      anchors,
      visualProjection.toLogical(visualValue),
    ).bucket.label;
    return Positioned(
      right: _controlWidth - 8,
      top: labelTop,
      child: IgnorePointer(
        child: Material(
          elevation: 2,
          color: colorScheme.inverseSurface,
          borderRadius: BorderRadius.circular(6),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
            child: Text(
              label,
              key: key,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: colorScheme.onInverseSurface,
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _updateHover(Offset localPosition, double railHeight) {
    if (_isDragging) {
      return;
    }
    if (localPosition.dx < _axisX - (_markerExtent * 2)) {
      if (_hoverValue != null) {
        setState(() => _hoverValue = null);
      }
      return;
    }
    final usableHeight = railHeight - (_endpointInset * 2);
    if (usableHeight <= 0) {
      return;
    }
    final nextValue = ((localPosition.dy - _endpointInset) / usableHeight)
        .clamp(0.0, 1.0)
        .toDouble();
    if (_hoverValue == null || (_hoverValue! - nextValue).abs() > 0.0001) {
      setState(() => _hoverValue = nextValue);
    }
  }

  void _step(int direction) {
    final onStep = widget.onStep;
    if (onStep != null) {
      onStep(direction);
      return;
    }
    widget.onChanged(
      (widget.value + (direction * 0.05)).clamp(0.0, 1.0).toDouble(),
    );
  }

  void _commit(double nextValue) {
    if (_isDragging || _hoverValue != null) {
      setState(() {
        _isDragging = false;
        _hoverValue = null;
      });
    }
    widget.onChangeEnd?.call(nextValue);
  }

  void _startInteraction(double value) {
    setState(() {
      _isDragging = true;
      _hoverValue = null;
    });
    widget.onChangeStart?.call(value);
  }

  bool _isUnlabeledMarkerVisible({
    required TimelineRailAnchor anchor,
    required double railHeight,
    required TimelineVisualProjection visualProjection,
    required Set<String> visibleMarkerIds,
    required Iterable<double> labelCenters,
  }) {
    if (!visibleMarkerIds.contains(anchor.bucket.id)) {
      return false;
    }
    final markerCenter = _topForValue(
      visualProjection.toVisual(anchor.value),
      railHeight,
    );
    final reservedDistance =
        (_labelExtent / 2) + (_markerExtent / 2) + _markerMinimumGap;
    return labelCenters.every(
      (labelCenter) => (labelCenter - markerCenter).abs() >= reservedDistance,
    );
  }

  static double _topForValue(double value, double railHeight) {
    final usableHeight = (railHeight - (_endpointInset * 2))
        .clamp(0.0, double.infinity)
        .toDouble();
    return _endpointInset + (value.clamp(0.0, 1.0) * usableHeight);
  }
}
