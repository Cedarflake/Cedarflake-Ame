import "library_organization_batch.dart";

class LibrarySelectionAvailability {
  const LibrarySelectionAvailability(this.summary);

  final LibraryBatchSummary summary;

  bool get canFavorite => summary.favorites < summary.groups;
  bool get canUnfavorite => summary.favorites > 0;
  bool get canRemoveAlbum => summary.assigned > 0 && summary.conflicts == 0;

  String get albumActionLabel {
    if (summary.assigned == 0) return "加入相册";
    if (summary.assigned == summary.groups) return "移至其他相册";
    return "加入或移至相册";
  }
}
