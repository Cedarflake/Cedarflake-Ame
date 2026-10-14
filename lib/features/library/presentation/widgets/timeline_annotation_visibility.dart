class TimelineAnnotationCandidate {
  const TimelineAnnotationCandidate({
    required this.id,
    required this.value,
    required this.extent,
  });

  final String id;
  final double value;
  final double extent;
}

class TimelineAnnotationPlacement {
  const TimelineAnnotationPlacement({required this.id, required this.center});

  final String id;
  final double center;
}

List<TimelineAnnotationPlacement> layoutTimelineAnnotations(
  List<TimelineAnnotationCandidate> candidates, {
  required double railExtent,
  required double startInset,
  required double endInset,
  double minimumGap = 8,
}) {
  if (candidates.isEmpty || railExtent <= 0) {
    return const [];
  }
  final visibleIds = visibleTimelineAnnotationIds(
    candidates,
    railExtent: railExtent,
    startInset: startInset,
    endInset: endInset,
    minimumGap: minimumGap,
  );
  final placements = _placements(
    [
      for (final candidate in candidates)
        if (visibleIds.contains(candidate.id)) candidate,
    ],
    railExtent: railExtent,
    startInset: startInset,
    endInset: endInset,
  );
  return List.unmodifiable([
    for (final placement in placements)
      TimelineAnnotationPlacement(
        id: placement.candidate.id,
        center: placement.top + (placement.candidate.extent / 2),
      ),
  ]);
}

Set<String> visibleTimelineAnnotationIds(
  List<TimelineAnnotationCandidate> candidates, {
  required double railExtent,
  required double startInset,
  required double endInset,
  double minimumGap = 8,
}) {
  if (candidates.isEmpty || railExtent <= 0) {
    return const {};
  }
  final placements = _placements(
    candidates,
    railExtent: railExtent,
    startInset: startInset,
    endInset: endInset,
  );
  final visibleIndices = <int>[];
  for (var index = placements.length - 1; index >= 0; index--) {
    final current = placements[index];
    if (current.top < 0 || current.bottom > railExtent) {
      continue;
    }
    if (visibleIndices.isEmpty) {
      visibleIndices.add(index);
      continue;
    }
    final lowerIndex = visibleIndices.last;
    final lower = placements[lowerIndex];
    if (lower.top >= current.bottom + minimumGap) {
      visibleIndices.add(index);
      continue;
    }
    final shouldKeepFirst = index == 0 && lowerIndex < placements.length - 1;
    if (shouldKeepFirst) {
      visibleIndices[visibleIndices.length - 1] = index;
    }
  }
  return {for (final index in visibleIndices) placements[index].candidate.id};
}

List<_AnnotationPlacement> _placements(
  List<TimelineAnnotationCandidate> candidates, {
  required double railExtent,
  required double startInset,
  required double endInset,
}) {
  final usableExtent = (railExtent - startInset - endInset)
      .clamp(0.0, double.infinity)
      .toDouble();
  return [
    for (final candidate in candidates)
      _AnnotationPlacement(
        candidate: candidate,
        top:
            startInset +
            (candidate.value.clamp(0.0, 1.0) * usableExtent) -
            (candidate.extent / 2),
      ),
  ];
}

class _AnnotationPlacement {
  _AnnotationPlacement({required this.candidate, required this.top});

  final TimelineAnnotationCandidate candidate;
  final double top;

  double get bottom => top + candidate.extent;
}
