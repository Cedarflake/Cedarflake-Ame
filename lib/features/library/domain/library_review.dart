import "library_models.dart";
import "library_organization.dart";

class LibraryReviewPosition {
  const LibraryReviewPosition(this.sessionId, this.sequence);
  final String sessionId;
  final BigInt sequence;

  LibraryReviewPosition next() =>
      LibraryReviewPosition(sessionId, sequence + BigInt.one);
}

enum LibraryContinuationStatus {
  noActivity,
  ready,
  anchorUnavailable,
  scopeUnavailable,
  emptyScope,
  endOfQueue,
}

class LibraryReviewContinuation {
  const LibraryReviewContinuation(this.status, {this.query, this.target});
  final LibraryContinuationStatus status;
  final LibraryGalleryQuery? query;
  final LibraryOrganizationTarget? target;
  bool get canResume => query != null && target != null;

  LibraryReviewContinuation forScope(LibraryGalleryQuery current) {
    final saved = query;
    if (saved == null) return this;
    if (current.collection is! LibraryAllCollection ||
        saved.collection is! LibraryAllCollection ||
        current.rootId != saved.rootId ||
        current.folderRelativePath != saved.folderRelativePath ||
        current.includeDescendants != saved.includeDescendants) {
      return const LibraryReviewContinuation(
        LibraryContinuationStatus.noActivity,
      );
    }
    return this;
  }
}

enum LibraryProgressAction { defer, ignore, restore }

sealed class LibraryReviewAction {
  const LibraryReviewAction();
}

class LibraryReviewAlbum extends LibraryReviewAction {
  const LibraryReviewAlbum(this.album);
  final LibraryOrganizationAlbum? album;
}

class LibraryReviewProgress extends LibraryReviewAction {
  const LibraryReviewProgress(this.action);
  final LibraryProgressAction action;
}

class LibraryReviewReceipt {
  const LibraryReviewReceipt(this.operation, this.position);
  final LibraryOrganizationReceipt operation;
  final LibraryReviewPosition position;
}
