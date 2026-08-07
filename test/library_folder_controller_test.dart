import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
      ),
    ]);
    final container = ProviderContainer(
      overrides: [libraryFolderCatalogProvider.overrideWithValue(catalog)],
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

  test("resets cached branches when the catalog revision changes", () async {
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
    ]);
    final container = ProviderContainer(
      overrides: [libraryFolderCatalogProvider.overrideWithValue(catalog)],
    );
    addTearDown(container.dispose);
    final controller = container.read(libraryFolderControllerProvider.notifier);

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
    expect(state.branch("root-1", "").hasLoaded, isFalse);
    expect(state.branch("root-2", "").folders, const [_rootTwoFolder]);
  });

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
      overrides: [libraryFolderCatalogProvider.overrideWithValue(catalog)],
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
    if (response is Exception) {
      throw response;
    }
    return response as LibraryFolderPage;
  }
}
