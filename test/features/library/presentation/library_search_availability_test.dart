import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_search_availability.dart";
import "package:flutter_test/flutter_test.dart";

const loading = LibraryQueryLoading(LibraryGalleryQuery(searchText: "20"));

void main() {
  test(
    "a query read does not block typing with no task or a retained task",
    () {
      expect(
        canEditLibrarySearch(const LibraryState(queryActivity: loading)),
        isTrue,
      );
      for (final status in [
        LibraryStatus.paused,
        LibraryStatus.failed,
        LibraryStatus.completed,
      ]) {
        expect(
          canEditLibrarySearch(
            LibraryState(
              queryActivity: loading,
              primaryScan: LibraryPrimaryScanSnapshot(
                status: status,
                taskKind: LibraryTaskKind.import,
                scanId: "retained",
              ),
            ),
          ),
          isTrue,
          reason: status.name,
        );
      }
    },
  );

  test("active primary commands remain blocked beside query loading", () {
    for (final status in [
      LibraryStatus.choosingDirectory,
      LibraryStatus.scanning,
      LibraryStatus.pausing,
      LibraryStatus.cancelling,
      LibraryStatus.refreshing,
    ]) {
      expect(
        canEditLibrarySearch(
          LibraryState(
            queryActivity: loading,
            primaryScan: LibraryPrimaryScanSnapshot(
              status: status,
              taskKind: LibraryTaskKind.import,
              scanId: "active",
            ),
          ),
        ),
        isFalse,
        reason: status.name,
      );
    }
  });

  test("uncommitted removal and failed committed display retain exclusion", () {
    expect(
      canEditLibrarySearch(
        const LibraryState(
          status: LibraryStatus.removing,
          taskKind: LibraryTaskKind.remove,
          queryActivity: loading,
        ),
      ),
      isFalse,
    );
    for (final activity in [loading, const LibraryQueryIdle()]) {
      expect(
        canEditLibrarySearch(
          LibraryState(
            status: LibraryStatus.failed,
            taskKind: LibraryTaskKind.remove,
            isRemovalCommitted: true,
            queryActivity: activity,
          ),
        ),
        isFalse,
      );
    }
  });

  test("time navigation retains its existing exclusion", () {
    for (final activity in [loading, const LibraryQueryIdle()]) {
      expect(
        canEditLibrarySearch(
          LibraryState(isLoadingTimeAnchor: true, queryActivity: activity),
        ),
        isFalse,
      );
    }
  });

  test("uncommitted removal failure permits another search", () {
    expect(
      canEditLibrarySearch(
        const LibraryState(
          status: LibraryStatus.failed,
          taskKind: LibraryTaskKind.remove,
          queryActivity: loading,
        ),
      ),
      isTrue,
    );
  });

  test("idle and failed queries preserve ordinary task availability", () {
    for (final status in [
      LibraryStatus.empty,
      LibraryStatus.completed,
      LibraryStatus.paused,
      LibraryStatus.failed,
      LibraryStatus.cancelled,
    ]) {
      expect(
        canEditLibrarySearch(LibraryState(status: status)),
        isTrue,
        reason: status.name,
      );
    }
    expect(
      canEditLibrarySearch(
        const LibraryState(
          queryActivity: LibraryQueryFailed(
            requestedQuery: LibraryGalleryQuery(searchText: "bad"),
            message: "unavailable",
          ),
        ),
      ),
      isTrue,
    );
    expect(
      canEditLibrarySearch(const LibraryState(status: LibraryStatus.scanning)),
      isFalse,
    );
    expect(
      canEditLibrarySearch(
        const LibraryState(status: LibraryStatus.refreshing),
      ),
      isFalse,
    );
  });
}
