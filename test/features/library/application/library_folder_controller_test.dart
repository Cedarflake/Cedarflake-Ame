import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "retains visible folders until an invalidated window is replaced",
    () async {
      final replacement = Completer<LibraryFolderPage>();
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
          nextCursor: LibraryFolderCursor(
            revision: BigInt.one,
            rootId: "root-1",
            parentRelativePath: "",
            relativePath: "Album",
          ),
        ),
        replacement.future,
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      final observed = <LibraryFolderBranch>[];
      container.listen(libraryFolderControllerProvider, (_, next) {
        observed.add(next.branch("root-1", ""));
      });

      final loading = controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      expect(catalog.requests.last.after, isNull);
      expect(observed, isNotEmpty);
      for (final branch in observed) {
        expect(branch.folders, const [_album]);
        expect(branch.revision, BigInt.one);
      }
      expect(observed.last.isLoading, isTrue);

      replacement.complete(
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
        ),
      );
      await loading;
      final branch = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(branch.folders, const [_other]);
      expect(branch.revision, BigInt.two);
      expect(branch.isLoading, isFalse);
    },
  );

  test(
    "accepts a coherent folder window newer than the displayed gallery",
    () async {
      final cursor = LibraryFolderCursor(
        revision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
        relativePath: "Album",
      );
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
          nextCursor: cursor,
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
          disposition: LibraryFolderPageDisposition.append,
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "")
            .errorMessage,
        isNull,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final branch = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(branch.revision, BigInt.two);
      expect(branch.folders, const [_album, _other]);
      expect(catalog.requests.last.after, same(cursor));
    },
  );

  test(
    "replaces a changed folder window without merging obsolete folders",
    () async {
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
          nextCursor: LibraryFolderCursor(
            revision: BigInt.one,
            rootId: "root-1",
            parentRelativePath: "",
            relativePath: "Album",
          ),
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final branch = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(branch.revision, BigInt.two);
      expect(branch.folders, const [_other]);
      expect(branch.errorMessage, isNull);
    },
  );

  test(
    "a gallery invalidation restarts load-more at a complete first window",
    () async {
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final branch = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(catalog.requests, hasLength(2));
      expect(catalog.requests.last.after, isNull);
      expect(branch.revision, BigInt.two);
      expect(branch.folders, const [_other]);
    },
  );

  test(
    "a late branch response cannot replace a newer invalidation generation",
    () async {
      final oldPage = Completer<LibraryFolderPage>();
      final catalog = _ScriptedFolderCatalog([
        oldPage.future,
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      final old = controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
      );
      oldPage.complete(
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
        ),
      );
      await old;
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "")
            .folders,
        const [_other],
      );
      expect(catalog.requests, hasLength(2));
    },
  );

  test(
    "rejects cross-revision append responses while preserving the loaded branch",
    () async {
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
          nextCursor: LibraryFolderCursor(
            revision: BigInt.one,
            rootId: "root-1",
            parentRelativePath: "",
            relativePath: "Album",
          ),
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
          disposition: LibraryFolderPageDisposition.append,
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final branch = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(branch.folders, const [_album]);
      expect(branch.errorMessage, contains("catalog_folder_page_inconsistent"));
    },
  );

  test("caches a loaded branch and appends bounded folder pages", () async {
    final firstCursor = LibraryFolderCursor(
      revision: BigInt.from(7),
      rootId: "root-1",
      parentRelativePath: "",
      relativePath: "Album",
    );
    final catalog = _ScriptedFolderCatalog([
      LibraryFolderPage(
        revision: BigInt.from(7),
        rootId: "root-1",
        parentRelativePath: "",
        folders: const [_album],
        nextCursor: firstCursor,
      ),
      LibraryFolderPage(
        revision: BigInt.from(7),
        rootId: "root-1",
        parentRelativePath: "",
        folders: const [_other],
        disposition: LibraryFolderPageDisposition.append,
      ),
    ]);
    final container = ProviderContainer(
      overrides: [
        libraryFolderCatalogProvider.overrideWithValue(catalog),
        libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
      ],
    );
    addTearDown(container.dispose);
    final controller = container.read(libraryFolderControllerProvider.notifier);

    await controller.loadBranch(
      catalogRevision: BigInt.from(7),
      rootId: "root-1",
      parentRelativePath: "",
    );
    await controller.loadBranch(
      catalogRevision: BigInt.from(7),
      rootId: "root-1",
      parentRelativePath: "",
    );

    expect(catalog.requests, hasLength(1));
    expect(
      container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "")
          .folders,
      const [_album],
    );

    await controller.loadBranch(
      catalogRevision: BigInt.from(7),
      rootId: "root-1",
      parentRelativePath: "",
      loadMore: true,
    );

    expect(catalog.requests, hasLength(2));
    expect(catalog.requests.last.after, same(firstCursor));
    expect(catalog.requests.last.maxItems, libraryFolderWindow);
    final branch = container
        .read(libraryFolderControllerProvider)
        .branch("root-1", "");
    expect(branch.folders, const [_album, _other]);
    expect(branch.hasMore, isFalse);
  });

  test(
    "retains peer windows without treating them as current after invalidation",
    () async {
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-2",
          parentRelativePath: "",
          folders: const [_rootTwoFolder],
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_other],
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );

      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-2",
        parentRelativePath: "",
      );

      final state = container.read(libraryFolderControllerProvider);
      expect(state.revision, BigInt.two);
      expect(state.branch("root-1", "").folders, const [_album]);
      expect(state.branch("root-1", "").revision, BigInt.one);
      expect(state.branch("root-2", "").folders, const [_rootTwoFolder]);
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
      );
      expect(catalog.requests, hasLength(3));
      expect(catalog.requests.last.after, isNull);
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "")
            .folders,
        const [_other],
      );
    },
  );

  test(
    "a failed replacement retains data and retries before publishing an empty window",
    () async {
      final catalog = _ScriptedFolderCatalog([
        LibraryFolderPage(
          revision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [_album],
        ),
        const LibraryCatalogFailure(
          code: "folder_read_failed",
          message: "Refresh failed",
        ),
        LibraryFolderPage(
          revision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
          folders: const [],
        ),
      ]);
      final container = ProviderContainer(
        overrides: [
          libraryFolderCatalogProvider.overrideWithValue(catalog),
          libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await controller.loadBranch(
        catalogRevision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
      );
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final failed = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(failed.folders, const [_album]);
      expect(failed.revision, BigInt.one);
      expect(failed.isLoading, isFalse);
      expect(failed.errorMessage, contains("folder_read_failed"));
      await controller.loadBranch(
        catalogRevision: BigInt.two,
        rootId: "root-1",
        parentRelativePath: "",
        loadMore: true,
      );
      final empty = container
          .read(libraryFolderControllerProvider)
          .branch("root-1", "");
      expect(
        catalog.requests.map((request) => request.after),
        everyElement(isNull),
      );
      expect(empty.folders, isEmpty);
      expect(empty.revision, BigInt.two);
      expect(empty.hasLoaded, isTrue);
      expect(empty.errorMessage, isNull);
    },
  );

  for (final oldFails in [false, true]) {
    test(
      "superseded branch ${oldFails ? 'failure' : 'success'} cannot release newer loading",
      () async {
        final oldPage = Completer<LibraryFolderPage>();
        final newPage = Completer<LibraryFolderPage>();
        final catalog = _ScriptedFolderCatalog([
          LibraryFolderPage(
            revision: BigInt.one,
            rootId: "root-1",
            parentRelativePath: "",
            folders: const [_album],
          ),
          oldPage.future,
          newPage.future,
        ]);
        final container = ProviderContainer(
          overrides: [
            libraryFolderCatalogProvider.overrideWithValue(catalog),
            libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
          ],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          libraryFolderControllerProvider.notifier,
        );
        await controller.loadBranch(
          catalogRevision: BigInt.one,
          rootId: "root-1",
          parentRelativePath: "",
        );
        final older = controller.loadBranch(
          catalogRevision: BigInt.two,
          rootId: "root-1",
          parentRelativePath: "",
        );
        final newer = controller.loadBranch(
          catalogRevision: BigInt.from(3),
          rootId: "root-1",
          parentRelativePath: "",
        );
        if (oldFails) {
          oldPage.completeError(
            const LibraryCatalogFailure(
              code: "old_read_failed",
              message: "Old read failed",
            ),
          );
        } else {
          oldPage.complete(
            LibraryFolderPage(
              revision: BigInt.two,
              rootId: "root-1",
              parentRelativePath: "",
              folders: const [],
            ),
          );
        }
        await older;
        final pending = container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "");
        expect(pending.folders, const [_album]);
        expect(pending.isLoading, isTrue);
        expect(pending.errorMessage, isNull);
        newPage.complete(
          LibraryFolderPage(
            revision: BigInt.from(3),
            rootId: "root-1",
            parentRelativePath: "",
            folders: const [_other],
          ),
        );
        await newer;
        final ready = container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "");
        expect(ready.folders, const [_other]);
        expect(ready.isLoading, isFalse);
      },
    );
  }

  test("keeps loaded folders when loading the next page fails", () async {
    final cursor = LibraryFolderCursor(
      revision: BigInt.one,
      rootId: "root-1",
      parentRelativePath: "",
      relativePath: "Album",
    );
    final catalog = _ScriptedFolderCatalog([
      LibraryFolderPage(
        revision: BigInt.one,
        rootId: "root-1",
        parentRelativePath: "",
        folders: const [_album],
        nextCursor: cursor,
      ),
      const LibraryCatalogFailure(
        code: "folder_page_failed",
        message: "无法读取下一页",
      ),
    ]);
    final container = ProviderContainer(
      overrides: [
        libraryFolderCatalogProvider.overrideWithValue(catalog),
        libraryFolderConfiguredRootsProvider.overrideWithValue(_roots),
      ],
    );
    addTearDown(container.dispose);
    final controller = container.read(libraryFolderControllerProvider.notifier);

    await controller.loadBranch(
      catalogRevision: BigInt.one,
      rootId: "root-1",
      parentRelativePath: "",
    );
    await controller.loadBranch(
      catalogRevision: BigInt.one,
      rootId: "root-1",
      parentRelativePath: "",
      loadMore: true,
    );

    final branch = container
        .read(libraryFolderControllerProvider)
        .branch("root-1", "");
    expect(branch.folders, const [_album]);
    expect(branch.hasLoaded, isTrue);
    expect(branch.hasMore, isTrue);
    expect(branch.errorMessage, contains("无法读取下一页"));
  });
}

const _roots = [
  LibraryRoot(
    id: "root-1",
    path: "C:\\Fixtures\\first",
    displayPath: "C:\\Fixtures\\first",
    createdUnixMs: 1,
    assetCount: 2,
    issueCount: 0,
    availability: LibraryRootAvailability.available,
  ),
  LibraryRoot(
    id: "root-2",
    path: "C:\\Fixtures\\second",
    displayPath: "C:\\Fixtures\\second",
    createdUnixMs: 1,
    assetCount: 1,
    issueCount: 0,
    availability: LibraryRootAvailability.available,
  ),
];

const _album = LibraryFolder(
  rootId: "root-1",
  relativePath: "Album",
  name: "Album",
  directAssetCount: 1,
  descendantAssetCount: 2,
);

const _other = LibraryFolder(
  rootId: "root-1",
  relativePath: "Other",
  name: "Other",
  directAssetCount: 1,
  descendantAssetCount: 1,
);

const _rootTwoFolder = LibraryFolder(
  rootId: "root-2",
  relativePath: "Second",
  name: "Second",
  directAssetCount: 1,
  descendantAssetCount: 1,
);

class _FolderRequest {
  const _FolderRequest({
    required this.rootId,
    required this.parentRelativePath,
    required this.maxItems,
    required this.after,
  });

  final String rootId;
  final String parentRelativePath;
  final int maxItems;
  final LibraryFolderCursor? after;
}

class _ScriptedFolderCatalog implements LibraryFolderCatalog {
  _ScriptedFolderCatalog(this.responses);

  final List<Object> responses;
  final List<_FolderRequest> requests = [];

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    requests.add(
      _FolderRequest(
        rootId: rootId,
        parentRelativePath: parentRelativePath,
        maxItems: maxItems,
        after: after,
      ),
    );
    final response = responses.removeAt(0);
    if (response is Future<LibraryFolderPage>) {
      return response;
    }
    if (response is Exception) {
      throw response;
    }
    return response as LibraryFolderPage;
  }
}
