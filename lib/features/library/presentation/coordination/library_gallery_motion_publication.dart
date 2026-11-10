import "../../domain/library_models.dart";
import "../../domain/library_state.dart";

class LibraryGalleryMotionPublication {
  const LibraryGalleryMotionPublication({
    required this.query,
    required this.layoutIdentity,
    required this.displayedItems,
    required this.sourceItems,
  });

  final LibraryGalleryQuery query;
  final Object layoutIdentity;
  final int displayedItems;
  final int sourceItems;

  static LibraryGalleryMotionPublication? forState(
    LibraryState state, {
    required Object layoutIdentity,
  }) {
    final timeline = state.timeline;
    if (timeline == null ||
        timeline.queryId != state.queryId ||
        timeline.revision != state.catalogRevision) {
      return null;
    }
    return LibraryGalleryMotionPublication(
      query: state.query,
      layoutIdentity: layoutIdentity,
      displayedItems: timeline.totalItems,
      sourceItems: timeline.totalSourceItems,
    );
  }

  bool sameScopeAs(LibraryGalleryMotionPublication previous) =>
      layoutIdentity == previous.layoutIdentity &&
      query.copyWith(duplicateMode: LibraryGalleryDuplicateMode.all) ==
          previous.query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.all,
          );

  bool changesDuplicatesFrom(LibraryGalleryMotionPublication previous) {
    if (!sameScopeAs(previous)) return false;
    if (query.foldsDuplicates != previous.query.foldsDuplicates) return true;
    return query.foldsDuplicates &&
        sourceItems >= previous.sourceItems &&
        sourceItems - displayedItems >
            previous.sourceItems - previous.displayedItems;
  }

  bool keepsDuplicatePresentation(LibraryGalleryMotionPublication previous) =>
      sameScopeAs(previous) &&
      query.foldsDuplicates == previous.query.foldsDuplicates &&
      displayedItems == previous.displayedItems &&
      sourceItems == previous.sourceItems;
}
