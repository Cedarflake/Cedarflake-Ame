import "../../application/library_browse_admission.dart";
import "../../application/library_page_operation.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";

typedef _PageAttempt = ({
  LibraryPageDirection direction,
  String queryId,
  BigInt? revision,
  LibraryCatalogCursor cursor,
});

/// Keeps edge demand across a busy read, without retrying unchanged cursors.
class LibraryGalleryPageDemand {
  LibraryGalleryQuery? _query;
  LibraryPageDirection? _direction;
  _PageAttempt? _attempt;

  bool get hasDemand => _direction != null;

  void scroll(LibraryGalleryQuery query, double delta) {
    if (delta == 0) return;
    final direction = delta < 0
        ? LibraryPageDirection.previous
        : LibraryPageDirection.next;
    if (_query != query || _direction != direction) clear();
    _query = query;
    _direction = direction;
  }

  void clear() {
    _query = null;
    _direction = null;
    _attempt = null;
  }

  LibraryPageDirection? requestFor(
    LibraryState state, {
    required bool nearStart,
    required bool nearEnd,
  }) {
    final direction = _direction;
    if (direction == null) return null;
    if (_query != state.query ||
        state.queryActivity is LibraryQueryFailed ||
        !(direction == LibraryPageDirection.previous ? nearStart : nearEnd)) {
      clear();
      return null;
    }
    final cursor = direction == LibraryPageDirection.previous
        ? state.previousCursor
        : state.nextCursor;
    final error = direction == LibraryPageDirection.previous
        ? state.previousPageErrorMessage
        : state.pageErrorMessage;
    if (cursor == null || error != null) {
      clear();
      return null;
    }
    if (!LibraryBrowseAdmission(state).canReadPage) return null;
    final attempt = (
      direction: direction,
      queryId: state.queryId,
      revision: state.catalogRevision,
      cursor: cursor,
    );
    if (_attempt == attempt) return null;
    _attempt = attempt;
    return direction;
  }
}
