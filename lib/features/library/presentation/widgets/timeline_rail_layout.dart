import "timeline_annotation_visibility.dart";
import "timeline_rail_model.dart";

class TimelineRailLayout {
  TimelineRailLayout({
    required List<TimelineRailAnchor> anchors,
    required double railExtent,
    required double endpointInset,
    required double labelExtent,
    required double markerExtent,
    required double markerMinimumGap,
  }) {
    final labels = layoutTimelineAnnotations(
      [
        for (final anchor in anchors)
          if (anchor.isYearStart ||
              anchor.bucket.isUnknown ||
              anchor.bucket.axisLabel != null)
            TimelineAnnotationCandidate(
              id: anchor.bucket.id,
              value: anchor.value,
              extent: labelExtent,
            ),
      ],
      railExtent: railExtent,
      startInset: endpointInset,
      endInset: endpointInset,
      minimumGap: 4,
    );
    labelIds = Set.unmodifiable(labels.map((label) => label.id));
    final usableExtent = (railExtent - endpointInset * 2)
        .clamp(0.0, double.infinity)
        .toDouble();
    final reservedDistance = markerExtent + markerMinimumGap;
    final markers = visibleTimelineAnnotationIds(
      [
        for (final anchor in anchors)
          if (!labelIds.contains(anchor.bucket.id) &&
              labels.every(
                (label) =>
                    (label.center -
                            (endpointInset + anchor.value * usableExtent))
                        .abs() >=
                    reservedDistance,
              ))
            TimelineAnnotationCandidate(
              id: anchor.bucket.id,
              value: anchor.value,
              extent: markerExtent,
            ),
      ],
      railExtent: railExtent,
      startInset: endpointInset,
      endInset: endpointInset,
      minimumGap: markerMinimumGap,
    );
    markerIds = Set.unmodifiable({...labelIds, ...markers});
  }

  late final Set<String> labelIds;
  late final Set<String> markerIds;
}
