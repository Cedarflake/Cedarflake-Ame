import "package:cedarflake_ame/features/library/application/library_browse_admission.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final status in [
    LibraryStatus.scanning,
    LibraryStatus.pausing,
    LibraryStatus.cancelling,
  ]) {
    test("$status requires a published catalog for browsing", () {
      final state = const LibraryState(
        roots: [retainedScanRoot],
      ).copyWith(status: status);
      final unpublished = LibraryBrowseAdmission(state);
      expect(unpublished.canBrowse, isFalse);
      expect(unpublished.canReadPage, isFalse);
      expect(unpublished.canNavigateTime, isFalse);
      expect(unpublished.canStartQuery(hasVisibleRangeRequest: false), isFalse);

      final published = LibraryBrowseAdmission(
        state.copyWith(roots: [retainedScanRoot, otherPublishedRoot]),
      );
      expect(published.canBrowse, isTrue);
      expect(published.canReadPage, isTrue);
      expect(published.canNavigateTime, isTrue);
      expect(published.canStartQuery(hasVisibleRangeRequest: false), isTrue);
      expect(state.isBusy, isTrue);
    });
  }

  test("pending query reads exclude paging but permit a newer query", () {
    final admission = LibraryBrowseAdmission(
      const LibraryState(roots: [otherPublishedRoot]).copyWith(
        queryActivity: const LibraryQueryLoading(
          LibraryGalleryQuery(rootId: "other"),
        ),
      ),
    );
    expect(admission.canBrowse, isFalse);
    expect(admission.canReadPage, isFalse);
    expect(admission.canNavigateTime, isFalse);
    expect(admission.canStartQuery(hasVisibleRangeRequest: false), isTrue);
  });

  test("a prepared removal or committed reload still excludes browsing", () {
    for (final state in [
      const LibraryState(roots: [otherPublishedRoot]).copyWith(
        status: LibraryStatus.removing,
        taskKind: LibraryTaskKind.remove,
      ),
      const LibraryState(roots: [otherPublishedRoot]).copyWith(
        status: LibraryStatus.failed,
        taskKind: LibraryTaskKind.remove,
        isRemovalCommitted: true,
      ),
    ]) {
      final admission = LibraryBrowseAdmission(state);
      expect(admission.canBrowse, isFalse);
      expect(admission.canReadPage, isFalse);
    }
  });

  test("time reads retain their explicit query supersession boundary", () {
    final admission = LibraryBrowseAdmission(
      const LibraryState(
        roots: [otherPublishedRoot],
        isLoadingTimeAnchor: true,
      ),
    );
    expect(admission.canBrowse, isFalse);
    expect(admission.canReadPage, isFalse);
    expect(admission.canNavigateTime, isFalse);
    expect(admission.canStartQuery(hasVisibleRangeRequest: false), isFalse);
    expect(admission.canStartQuery(hasVisibleRangeRequest: true), isTrue);
  });
}
