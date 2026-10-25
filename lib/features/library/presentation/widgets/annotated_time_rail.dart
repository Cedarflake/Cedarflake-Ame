import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "timeline_linear_projection.dart";
import "timeline_rail_layout.dart";
import "timeline_rail_model.dart";
import "timeline_rail_motion.dart";
import "timeline_rail_ticks.dart";
import "timeline_rail_ranges.dart";
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
    this.interactionKey,
    super.key,
  });

  final double value;
  final List<TimelineRailBucket> buckets;
  final double? maximumScrollOffset;
  final TimelineLinearProjection? projection;
  final ValueChanged<double>? onChanged;
  final ValueChanged<double>? onChangeStart;
  final ValueChanged<double>? onChangeEnd;
  final ValueChanged<int>? onStep;
  final Key? interactionKey;

  static const double width = 80;

  @override
  State<AnnotatedTimeRail> createState() => _AnnotatedTimeRailState();
}

class _AnnotatedTimeRailState extends State<AnnotatedTimeRail> {
  static const double _controlWidth = kMinInteractiveDimension;
  static const double _endpointInset = 12;
  static const double _markerExtent = TimelineRailMarker.extent;
  static const double _markerMinimumGap = TimelineRailMarker.minimumGap;
  static const double _labelExtent = 24;
  static const double _positionLineHeight = 3;
  static const double _positionLineWidth = 30;
  static const double _axisX = AnnotatedTimeRail.width - (_controlWidth / 2);

  double? _hoverValue;
  bool _isDragging = false;

  @override
  void didUpdateWidget(covariant AnnotatedTimeRail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.onChanged == null ||
        widget.interactionKey != oldWidget.interactionKey) {
      _hoverValue = null;
      _isDragging = false;
    }
  }

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
    if (anchors.isEmpty && widget.maximumScrollOffset == null) {
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
              tooltip: "向上滚动",
              icon: Symbols.arrow_drop_up_rounded,
              direction: -1,
            ),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final geometry = TimelineRailGeometry(
                    extent: constraints.maxHeight,
                    maximumOffset: projection.maximumOffset,
                    anchors: anchors,
                  );
                  final layout = TimelineRailLayout(
                    anchors: anchors,
                    railExtent: constraints.maxHeight,
                    endpointInset: _endpointInset,
                    labelExtent: _labelExtent,
                    markerExtent: _markerExtent,
                    markerMinimumGap: _markerMinimumGap,
                  );
                  final currentBucketId = anchors.isEmpty
                      ? null
                      : timelineRailBucketAtValue(
                          anchors,
                          widget.value,
                        ).bucket.id;
                  return Listener(
                    key: const Key("timeline-hover-region"),
                    behavior: HitTestBehavior.translucent,
                    onPointerHover: widget.onChanged == null
                        ? null
                        : (event) => _updateHover(
                            event.localPosition,
                            constraints.maxHeight,
                          ),
                    child: MouseRegion(
                      cursor: widget.onChanged == null
                          ? MouseCursor.defer
                          : SystemMouseCursors.click,
                      onExit: (_) => setState(() => _hoverValue = null),
                      child: Stack(
                        clipBehavior: Clip.none,
                        children: [
                          Positioned.fill(
                            child: TimelineRailRanges(
                              anchors: anchors,
                              projection: projection,
                              geometry: geometry,
                              endpointInset: _endpointInset,
                              isInteracting: _isDragging,
                            ),
                          ),
                          Positioned(
                            top: 0,
                            right: 0,
                            bottom: 0,
                            width: _controlWidth,
                            child: VerticalMaterialTimelineSlider(
                              key: widget.interactionKey,
                              value: widget.value,
                              endpointInset: _endpointInset,
                              onChangeStart: widget.onChanged == null
                                  ? null
                                  : (visualValue) =>
                                        _startInteraction(visualValue),
                              onChanged: widget.onChanged == null
                                  ? null
                                  : (visualValue) =>
                                        widget.onChanged?.call(visualValue),
                              onChangeEnd: widget.onChanged == null
                                  ? null
                                  : (visualValue) => _commit(visualValue),
                              semanticLabelFor: (sliderValue) => anchors.isEmpty
                                  ? "滚动位置 ${(sliderValue * 100).round()}%"
                                  : timelineRailBucketAtValue(
                                      anchors,
                                      sliderValue,
                                    ).bucket.label,
                            ),
                          ),
                          _buildPositionLine(
                            key: const Key("timeline-current-line"),
                            value: widget.value,
                            railHeight: constraints.maxHeight,
                            color: colorScheme.primary,
                            geometry: geometry,
                          ),
                          if (anchors.isEmpty)
                            Positioned(
                              top: 0,
                              bottom: 0,
                              right: (_controlWidth - _markerExtent) / 2,
                              width: _markerExtent,
                              child: const TimelineRailTicks(
                                endpointInset: _endpointInset,
                              ),
                            ),
                          for (final anchor in anchors)
                            ..._buildAnnotation(
                              context,
                              anchor: anchor,
                              geometry: geometry,
                              railHeight: constraints.maxHeight,
                              visualValue: anchor.value,
                              hasLabel: layout.labelIds.contains(
                                anchor.bucket.id,
                              ),
                              hasMarker: layout.markerIds.contains(
                                anchor.bucket.id,
                              ),
                              isCurrent: currentBucketId == anchor.bucket.id,
                            ),
                          if (anchors.isNotEmpty && _isDragging)
                            _buildPositionLabel(
                              context,
                              key: const Key("timeline-drag-label"),
                              anchors: anchors,
                              visualValue: widget.value,
                              railHeight: constraints.maxHeight,
                            )
                          else if (anchors.isNotEmpty && _hoverValue != null)
                            ..._buildHoverPreview(
                              context,
                              anchors: anchors,
                              visualValue: _hoverValue!,
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
              tooltip: "向下滚动",
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
        child: AmeTooltip(
          message: tooltip,
          child: IconButton(
            key: key,
            onPressed: widget.onChanged == null ? null : () => _step(direction),
            icon: Icon(icon),
          ),
        ),
      ),
    );
  }

  List<Widget> _buildAnnotation(
    BuildContext context, {
    required TimelineRailAnchor anchor,
    required TimelineRailGeometry geometry,
    required double railHeight,
    required double visualValue,
    required bool hasLabel,
    required bool hasMarker,
    required bool isCurrent,
  }) {
    final markerCenter = _topForValue(visualValue, railHeight);
    final marker = TimelineRailPosition(
      key: ValueKey("time-marker-${anchor.bucket.id}"),
      geometry: geometry,
      isInteracting: _isDragging,
      rect: Rect.fromLTWH(
        _axisX - (_markerExtent / 2),
        markerCenter - (_markerExtent / 2),
        _markerExtent,
        _markerExtent,
      ),
      child: const TimelineRailMarker(),
    );
    if (!hasLabel) {
      if (!hasMarker) {
        return const [];
      }
      return [marker];
    }
    return [
      if (hasMarker) marker,
      TimelineRailPosition(
        key: ValueKey("time-label-${anchor.bucket.id}"),
        geometry: geometry,
        isInteracting: _isDragging,
        rect: Rect.fromLTWH(
          2,
          markerCenter - (_labelExtent / 2),
          40,
          _labelExtent,
        ),
        child: AmeTooltip(
          message: anchor.bucket.label,
          child: Align(
            alignment: Alignment.centerRight,
            child: Text(
              anchor.bucket.axisLabel ??
                  (anchor.bucket.isUnknown ? "未知" : "${anchor.bucket.year}"),
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

  Widget _buildPositionLine({
    required Key key,
    required double value,
    required double railHeight,
    required Color color,
    TimelineRailGeometry? geometry,
  }) {
    final rect = Rect.fromLTWH(
      _axisX - (_positionLineWidth / 2),
      _topForValue(value, railHeight) - (_positionLineHeight / 2),
      _positionLineWidth,
      _positionLineHeight,
    );
    final line = IgnorePointer(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(_positionLineHeight / 2),
        ),
      ),
    );
    if (geometry == null) {
      return Positioned.fromRect(key: key, rect: rect, child: line);
    }
    return TimelineRailPosition(
      key: key,
      rect: rect,
      geometry: geometry,
      isInteracting: _isDragging,
      child: line,
    );
  }

  List<Widget> _buildHoverPreview(
    BuildContext context, {
    required List<TimelineRailAnchor> anchors,
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
        visualValue: visualValue,
        railHeight: railHeight,
      ),
    ];
  }

  Positioned _buildPositionLabel(
    BuildContext context, {
    required Key key,
    required List<TimelineRailAnchor> anchors,
    required double visualValue,
    required double railHeight,
  }) {
    final colorScheme = Theme.of(context).colorScheme;
    final lineTop = _topForValue(visualValue, railHeight);
    final labelTop = (lineTop - 14).clamp(0.0, railHeight - 28).toDouble();
    final label = timelineRailBucketAtValue(anchors, visualValue).bucket.label;
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
    widget.onChanged?.call(
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

  static double _topForValue(double value, double railHeight) {
    final usableHeight = (railHeight - (_endpointInset * 2))
        .clamp(0.0, double.infinity)
        .toDouble();
    return _endpointInset + (value.clamp(0.0, 1.0) * usableHeight);
  }
}
