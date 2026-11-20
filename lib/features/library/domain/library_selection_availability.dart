import "../../../app/messages/ame_message.dart";
import "library_organization_batch.dart";

class LibrarySelectionAvailability {
  const LibrarySelectionAvailability(this.summary);

  final LibraryBatchSummary summary;

  bool get canFavorite => summary.favorites < summary.groups;
  bool get canUnfavorite => summary.favorites > 0;
  bool get canRemoveAlbum => summary.assigned > 0 && summary.conflicts == 0;

  AmeMessage get albumActionLabel {
    if (summary.assigned == 0) return AmeLabel.addToAlbum;
    if (summary.assigned == summary.groups) return AmeLabel.moveToAnotherAlbum;
    return AmeLabel.addOrMoveToAlbum;
  }
}
