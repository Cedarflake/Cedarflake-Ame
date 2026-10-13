import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final removesParent in [false, true]) {
    test(
      "parent publication retires ${removesParent ? 'removed' : 'leaf'} descendants and their late result",
      () async {
        final pendingChild = Completer<LibraryFolderPage>();
        final catalog = _Catalog([
          _page(1, "", [_folder("Album", children: true)]),
          _page(1, "Album", [_folder("Album/Old")]),
          pendingChild.future,
          _page(2, "", removesParent ? [] : [_folder("Album")]),
        ]);
        final container = _container(catalog);
        addTearDown(container.dispose);
        final controller = container.read(
          libraryFolderControllerProvider.notifier,
        );
        await _load(controller, "");
        await _load(controller, "Album");
        final child = _load(controller, "Album", force: true);
        await _load(controller, "", force: true);
        expect(
          container
              .read(libraryFolderControllerProvider)
              .branches
              .containsKey(_albumKey),
          isFalse,
        );
        pendingChild.complete(_page(2, "Album", [_folder("Album/Late")]));
        await child;
        expect(
          container
              .read(libraryFolderControllerProvider)
              .branches
              .containsKey(_albumKey),
          isFalse,
        );
      },
    );
  }

  test(
    "an incomplete replacement does not prove an unseen subtree was removed",
    () async {
      final catalog = _Catalog([
        _page(1, "", [_folder("Album", children: true)]),
        _page(1, "Album", [_folder("Album/Old")]),
        _page(2, "", [_folder("Other")], hasMore: true),
        _page(2, "", [_folder("Album", children: true)], append: true),
        _page(2, "Album", [_folder("Album/Current")]),
      ]);
      final container = _container(catalog);
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await _load(controller, "");
      await _load(controller, "Album");
      await _load(controller, "", revision: 2);
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "Album")
            .folders
            .single
            .relativePath,
        "Album/Old",
      );
      await _load(controller, "", revision: 2, loadMore: true);
      await _load(controller, "Album", revision: 2);
      expect(catalog.requests, hasLength(5));
      expect(catalog.requests.last, isNull);
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branch("root-1", "Album")
            .folders
            .single
            .relativePath,
        "Album/Current",
      );
    },
  );

  test(
    "removing a root retires its windows and pending reads without changing a peer",
    () async {
      final pending = Completer<LibraryFolderPage>();
      final catalog = _Catalog([
        _page(1, "", [_folder("Album", children: true)]),
        _page(1, "Album", [_folder("Album/Old")]),
        _page(1, "", [_folder("Peer", rootId: "root-2")], rootId: "root-2"),
        pending.future,
      ]);
      final container = _container(catalog);
      addTearDown(container.dispose);
      final controller = container.read(
        libraryFolderControllerProvider.notifier,
      );
      await _load(controller, "");
      await _load(controller, "Album");
      await _load(controller, "", rootId: "root-2");
      final older = _load(controller, "Album", force: true);
      container.read(_rootsProvider.notifier).removeFirst();
      await container.pump();
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branches
            .keys
            .map((key) => key.rootId),
        ["root-2"],
      );
      await _load(controller, "");
      expect(catalog.requests, hasLength(4));
      pending.complete(_page(1, "Album", [_folder("Album/Late")]));
      await older;
      expect(
        container
            .read(libraryFolderControllerProvider)
            .branches
            .keys
            .map((key) => key.rootId),
        ["root-2"],
      );
    },
  );

  test("disposing the owner rejects a late branch read", () async {
    final page = Completer<LibraryFolderPage>();
    final container = _container(_Catalog([page.future]));
    final loading = _load(
      container.read(libraryFolderControllerProvider.notifier),
      "",
    );
    container.dispose();
    page.complete(_page(1, "", [_folder("Late")]));
    await loading;
  });
}

Future<void> _load(
  LibraryFolderController controller,
  String parent, {
  int revision = 1,
  String rootId = "root-1",
  bool loadMore = false,
  bool force = false,
}) => controller.loadBranch(
  catalogRevision: BigInt.from(revision),
  rootId: rootId,
  parentRelativePath: parent,
  loadMore: loadMore,
  force: force,
);

ProviderContainer _container(_Catalog catalog) => ProviderContainer(
  overrides: [
    libraryFolderCatalogProvider.overrideWithValue(catalog),
    libraryFolderConfiguredRootsProvider.overrideWith(
      (ref) => ref.watch(_rootsProvider),
    ),
  ],
);

const _albumKey = LibraryFolderBranchKey(
  rootId: "root-1",
  parentRelativePath: "Album",
);
final _rootsProvider = NotifierProvider<_Roots, List<LibraryRoot>>(_Roots.new);

class _Roots extends Notifier<List<LibraryRoot>> {
  @override
  List<LibraryRoot> build() => [
    for (final id in ["root-1", "root-2"])
      LibraryRoot(
        id: id,
        path: "C:\\Fixtures\\$id",
        displayPath: "C:\\Fixtures\\$id",
        createdUnixMs: 1,
        assetCount: 2,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
  ];

  void removeFirst() => state = [state.last];
}

LibraryFolder _folder(
  String path, {
  bool children = false,
  String rootId = "root-1",
}) => LibraryFolder(
  rootId: rootId,
  relativePath: path,
  name: path.split("/").last,
  directAssetCount: 1,
  descendantAssetCount: children ? 2 : 1,
);

LibraryFolderPage _page(
  int revision,
  String parent,
  List<LibraryFolder> folders, {
  bool hasMore = false,
  bool append = false,
  String rootId = "root-1",
}) => LibraryFolderPage(
  revision: BigInt.from(revision),
  rootId: rootId,
  parentRelativePath: parent,
  folders: folders,
  nextCursor: hasMore
      ? LibraryFolderCursor(
          revision: BigInt.from(revision),
          rootId: rootId,
          parentRelativePath: parent,
          relativePath: folders.last.relativePath,
        )
      : null,
  disposition: append
      ? LibraryFolderPageDisposition.append
      : LibraryFolderPageDisposition.replace,
);

class _Catalog implements LibraryFolderCatalog {
  _Catalog(this.responses);
  final List<Object> responses;
  final requests = <LibraryFolderCursor?>[];

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    requests.add(after);
    final response = responses.removeAt(0);
    if (response is Future<LibraryFolderPage>) {
      return response;
    }
    return response as LibraryFolderPage;
  }
}
