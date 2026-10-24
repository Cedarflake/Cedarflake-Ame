import "library_models.dart";
import "library_organization.dart";

class LibraryOrganizationSelection {
  LibraryOrganizationSelection({
    required this.query,
    required this.catalogRevision,
    required this.allMatching,
    required Iterable<String> assetIds,
  }) : assetIds = List.unmodifiable(assetIds);

  final LibraryGalleryQuery query;
  final BigInt catalogRevision;
  final bool allMatching;
  final List<String> assetIds;
}

class LibraryBatchSummary {
  const LibraryBatchSummary({
    required this.groups,
    required this.favorites,
    required this.assigned,
    required this.conflicts,
    this.commonAlbum,
  });
  final int groups;
  final int favorites;
  final int assigned;
  final int conflicts;
  final LibraryOrganizationAlbum? commonAlbum;
}

sealed class LibraryBatchAction {
  const LibraryBatchAction();
}

class LibraryBatchFavorite extends LibraryBatchAction {
  const LibraryBatchFavorite(this.favorite);
  final bool favorite;
}

class LibraryBatchAlbum extends LibraryBatchAction {
  const LibraryBatchAlbum(this.albumId);
  final String? albumId;
}

enum LibraryBatchPhase {
  collecting,
  preparing,
  ready,
  saving,
  completed,
  failed,
  cancelled,
}

class LibraryBatchProgress {
  const LibraryBatchProgress(this.phase, this.completed, this.total);
  final LibraryBatchPhase phase;
  final int completed;
  final int total;
}
