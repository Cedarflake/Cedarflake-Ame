import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_root_removal_page_reader.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final fails in [false, true]) {
    test(
      "disposed removal read cannot publish or load a timeline: $fails",
      () async {
        final pending = Completer<LibrarySnapshot>();
        final catalog = _Catalog(pending.future);
        final initial = LibraryState.fromSnapshot(_snapshot(1));
        var state = initial;
        final viewport = LibraryViewportController(
          catalog,
          () => state,
          (value) => state = value,
          () => false,
          (_) {},
        );
        addTearDown(viewport.dispose);
        viewport.seed(state);
        final read = viewport.reloadCommittedRootRemovalFirstPage(
          removedRootId: "removed",
        );
        expect(catalog.pageReads, 1);
        viewport.dispose();
        if (fails) {
          pending.completeError(StateError("retired page failure"));
        } else {
          pending.complete(_snapshot(2));
        }
        expect(await read, isFalse);
        expect(state, same(initial));
        expect(catalog.timelineReads, 0);
      },
    );
  }

  test("a current removal page cannot retain the unregistered root", () async {
    final catalog = _Catalog(Future.value(_snapshot(2, rootId: "removed")));
    final reader = LibraryRootRemovalPageReader(catalog);
    await expectLater(
      reader.loadCurrent(
        removedRootId: "removed",
        query: const LibraryGalleryQuery(),
        anchor: null,
        canPublish: () => true,
      ),
      throwsA(
        isA<LibraryCatalogFailure>().having(
          (error) => error.code,
          "code",
          "catalog_root_removal_not_visible",
        ),
      ),
    );
    expect(catalog.pageReads, 1);
    expect(catalog.timelineReads, 0);
  });
}

LibrarySnapshot _snapshot(int revision, {String rootId = "peer"}) =>
    LibrarySnapshot(
      catalogPath: "removal-page.sqlite3",
      revision: BigInt.from(revision),
      queryId: "query",
      roots: [
        LibraryRoot(
          id: rootId,
          path: "C:\\Generated\\$rootId",
          displayPath: "C:\\Generated\\$rootId",
          activeScanId: "$rootId-scan",
          createdUnixMs: 1,
          assetCount: 0,
          issueCount: 0,
        ),
      ],
      assets: const [],
    );

class _Catalog extends Fake implements LibraryCatalog {
  _Catalog(this.page);

  final Future<LibrarySnapshot> page;
  int pageReads = 0;
  int timelineReads = 0;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) {
    expect(maxItems, libraryCatalogWindow);
    pageReads++;
    return page;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    timelineReads++;
    throw StateError("A retired removal page cannot start a timeline read");
  }
}
