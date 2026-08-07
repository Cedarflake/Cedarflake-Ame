class GallerySelection {
  const GallerySelection._({
    required this.queryId,
    required this.isAllMatching,
    required this.includedLocationIds,
    required this.excludedLocationIds,
  });

  factory GallerySelection.empty(String queryId) {
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: false,
      includedLocationIds: const {},
      excludedLocationIds: const {},
    );
  }

  final String queryId;
  final bool isAllMatching;
  final Set<String> includedLocationIds;
  final Set<String> excludedLocationIds;

  bool get isEmpty => !isAllMatching && includedLocationIds.isEmpty;

  int selectedCount(int totalMatching) {
    if (isAllMatching) {
      return (totalMatching - excludedLocationIds.length).clamp(
        0,
        totalMatching,
      );
    }
    return includedLocationIds.length;
  }

  bool contains(String locationId) {
    if (isAllMatching) {
      return !excludedLocationIds.contains(locationId);
    }
    return includedLocationIds.contains(locationId);
  }

  GallerySelection toggle(String locationId) {
    if (isAllMatching) {
      final exclusions = {...excludedLocationIds};
      if (!exclusions.add(locationId)) {
        exclusions.remove(locationId);
      }
      return GallerySelection._(
        queryId: queryId,
        isAllMatching: true,
        includedLocationIds: const {},
        excludedLocationIds: Set.unmodifiable(exclusions),
      );
    }
    final inclusions = {...includedLocationIds};
    if (!inclusions.add(locationId)) {
      inclusions.remove(locationId);
    }
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: false,
      includedLocationIds: Set.unmodifiable(inclusions),
      excludedLocationIds: const {},
    );
  }

  GallerySelection selectAll() {
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: true,
      includedLocationIds: const {},
      excludedLocationIds: const {},
    );
  }

  GallerySelection clear() => GallerySelection.empty(queryId);
}
