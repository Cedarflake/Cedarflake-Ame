class GallerySelection {
  const GallerySelection._({
    required this.queryId,
    required this.isAllMatching,
    required this.includedAssetIds,
    required this.excludedAssetIds,
  });

  factory GallerySelection.empty(String queryId) {
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: false,
      includedAssetIds: const {},
      excludedAssetIds: const {},
    );
  }

  final String queryId;
  final bool isAllMatching;
  final Set<String> includedAssetIds;
  final Set<String> excludedAssetIds;

  bool get isEmpty => !isAllMatching && includedAssetIds.isEmpty;

  int selectedCount(int totalMatching) {
    if (isAllMatching) {
      return (totalMatching - excludedAssetIds.length).clamp(0, totalMatching);
    }
    return includedAssetIds.length;
  }

  bool contains(String assetId) {
    if (isAllMatching) {
      return !excludedAssetIds.contains(assetId);
    }
    return includedAssetIds.contains(assetId);
  }

  GallerySelection toggle(String assetId) {
    if (isAllMatching) {
      final exclusions = {...excludedAssetIds};
      if (!exclusions.add(assetId)) {
        exclusions.remove(assetId);
      }
      return GallerySelection._(
        queryId: queryId,
        isAllMatching: true,
        includedAssetIds: const {},
        excludedAssetIds: Set.unmodifiable(exclusions),
      );
    }
    final inclusions = {...includedAssetIds};
    if (!inclusions.add(assetId)) {
      inclusions.remove(assetId);
    }
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: false,
      includedAssetIds: Set.unmodifiable(inclusions),
      excludedAssetIds: const {},
    );
  }

  GallerySelection selectAll() {
    return GallerySelection._(
      queryId: queryId,
      isAllMatching: true,
      includedAssetIds: const {},
      excludedAssetIds: const {},
    );
  }

  GallerySelection clear() => GallerySelection.empty(queryId);

  GallerySelection rebind(String nextQueryId) {
    if (isAllMatching) {
      return GallerySelection.empty(nextQueryId);
    }
    return GallerySelection._(
      queryId: nextQueryId,
      isAllMatching: false,
      includedAssetIds: includedAssetIds,
      excludedAssetIds: const {},
    );
  }
}
