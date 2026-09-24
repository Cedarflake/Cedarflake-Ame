import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "a refreshed leaf retires an expanded child and can be expanded again after recovery",
    (tester) async {
      final catalog = _NestedCatalog();
      final revision = ValueNotifier(BigInt.one);
      addTearDown(revision.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryFolderCatalogProvider.overrideWithValue(catalog),
            libraryFolderConfiguredRootsProvider.overrideWithValue(const [
              _root,
            ]),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: ValueListenableBuilder(
                valueListenable: revision,
                builder: (_, value, _) => _Navigation(revision: value),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const ValueKey("source-expand-root-1")));
      await tester.pumpAndSettle();
      final expand = find.byKey(const ValueKey("folder-expand-root-1-Album"));
      await tester.tap(expand);
      await tester.pumpAndSettle();
      expect(find.text("Old child"), findsOneWidget);

      revision.value = BigInt.two;
      await tester.pump();
      await tester.tap(find.byKey(const Key("folder-tree-load-more")));
      await tester.pumpAndSettle();
      expect(find.text("Old child"), findsNothing);
      expect(expand, findsNothing);

      revision.value = BigInt.from(3);
      await tester.pump();
      await tester.tap(find.byKey(const Key("folder-tree-load-more")));
      await tester.pumpAndSettle();
      expect(expand, findsOneWidget);
      await tester.tap(expand);
      await tester.pumpAndSettle();
      expect(find.text("Current child"), findsOneWidget);
      expect(find.text("Old child"), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  for (final outcome in ["replacement", "failure", "empty"]) {
    testWidgets("Show More retains sidebar position until $outcome settles", (
      tester,
    ) async {
      final catalog = _FolderCatalog();
      final revision = ValueNotifier(BigInt.one);
      addTearDown(revision.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryFolderCatalogProvider.overrideWithValue(catalog),
            libraryFolderConfiguredRootsProvider.overrideWithValue(const [
              _root,
            ]),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: ValueListenableBuilder(
                valueListenable: revision,
                builder: (_, currentRevision, _) =>
                    _Navigation(revision: currentRevision),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const ValueKey("source-expand-root-1")));
      await tester.pumpAndSettle();
      final more = find.byKey(const Key("folder-tree-load-more"));
      await tester.scrollUntilVisible(more, 400, maxScrolls: 60);
      await tester.pumpAndSettle();
      final scroll = tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position;
      final before = scroll.pixels;
      expect(before, greaterThan(1000));
      final lastFolder = find.byKey(
        const ValueKey("folder-tile-root-1-album-199"),
      );
      final beforePosition = tester.getTopLeft(lastFolder);

      revision.value = BigInt.two;
      await tester.pump();
      await tester.tap(more);
      await tester.pump();
      expect(catalog.requests.last, isNull);
      expect(scroll.pixels, closeTo(before, 0.01));
      expect(tester.getTopLeft(lastFolder), beforePosition);

      if (outcome == "failure") {
        catalog.replacement.completeError(
          const LibraryCatalogFailure(
            code: "folder_read_failed",
            message: "Refresh failed",
          ),
        );
        await tester.pumpAndSettle();
        expect(scroll.pixels, closeTo(before, 0.01));
        expect(tester.getTopLeft(lastFolder), beforePosition);
        expect(more, findsNothing);
        await tester.tap(find.byKey(const Key("folder-tree-retry")));
        await tester.pumpAndSettle();
        expect(catalog.requests.last, isNull);
        expect(find.byKey(const Key("folder-tree-retry")), findsNothing);
      } else if (outcome == "empty") {
        catalog.replacement.complete(
          LibraryFolderPage(
            revision: BigInt.two,
            rootId: "root-1",
            parentRelativePath: "",
            folders: const [],
          ),
        );
        await tester.pumpAndSettle();
        expect(scroll.pixels, 0);
        expect(lastFolder, findsNothing);
        expect(more, findsNothing);
        expect(tester.takeException(), isNull);
        return;
      } else {
        catalog.replacement.complete(
          _page(BigInt.two, firstName: "Replacement"),
        );
      }
      await tester.pumpAndSettle();
      expect(scroll.pixels, closeTo(before, 0.01));
      expect(tester.getTopLeft(lastFolder), beforePosition);
      await tester.tap(more);
      await tester.pumpAndSettle();
      expect(catalog.requests.last?.revision, BigInt.two);
      expect(scroll.pixels, closeTo(before, 0.01));
      expect(tester.takeException(), isNull);
    });
  }
}

class _Navigation extends ConsumerWidget {
  const _Navigation({required this.revision});

  final BigInt revision;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tree = ref.watch(libraryFolderControllerProvider);
    final controller = ref.read(libraryFolderControllerProvider.notifier);
    return Align(
      alignment: Alignment.topLeft,
      child: LibraryNavigation(
        isCompact: false,
        width: 260,
        isSettingsSelected: false,
        roots: const [_root],
        selectedRootId: "root-1",
        selectedFolderRelativePath: null,
        transientRootPath: null,
        folderTree: tree,
        isBusy: false,
        isBrowseDisabled: false,
        onSelectLibrary: () {},
        onSelectRoot: (_) {},
        onSelectFolder: (_, _) {},
        onExpandFolder: (rootId, path) => controller.loadBranch(
          catalogRevision: revision,
          rootId: rootId,
          parentRelativePath: path,
        ),
        onLoadMoreFolders: (rootId, path) => controller.loadBranch(
          catalogRevision: revision,
          rootId: rootId,
          parentRelativePath: path,
          loadMore: true,
        ),
        onAddSource: () {},
        onOpenSettings: () {},
        onUpdateRoot: (_) {},
        onOpenRoot: (_) {},
        onOpenFolder: (_, _) {},
        onRemoveRoot: (_) {},
      ),
    );
  }
}

const _root = LibraryRoot(
  id: "root-1",
  path: "C:\\Fixtures",
  displayPath: "C:\\Fixtures",
  createdUnixMs: 1,
  assetCount: 240,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

LibraryFolder _folder(int index, {String? name}) => LibraryFolder(
  rootId: "root-1",
  relativePath: "album-$index",
  name: name ?? "Album $index",
  directAssetCount: 1,
  descendantAssetCount: 1,
);

LibraryFolderPage _page(BigInt revision, {String? firstName}) =>
    LibraryFolderPage(
      revision: revision,
      rootId: "root-1",
      parentRelativePath: "",
      folders: [
        for (var index = 0; index < libraryFolderWindow; index++)
          _folder(index, name: index == 0 ? firstName : null),
      ],
      nextCursor: LibraryFolderCursor(
        revision: revision,
        rootId: "root-1",
        parentRelativePath: "",
        relativePath: "album-199",
      ),
    );

class _FolderCatalog implements LibraryFolderCatalog {
  final replacement = Completer<LibraryFolderPage>();
  final requests = <LibraryFolderCursor?>[];

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    requests.add(after);
    expect(maxItems, libraryFolderWindow);
    if (requests.length == 1) {
      return _page(BigInt.one);
    }
    if (requests.length == 2) {
      return replacement.future;
    }
    if (after == null) {
      return _page(BigInt.two, firstName: "Replacement");
    }
    return LibraryFolderPage(
      revision: BigInt.two,
      rootId: rootId,
      parentRelativePath: parentRelativePath,
      folders: [for (var index = 200; index < 240; index++) _folder(index)],
      disposition: LibraryFolderPageDisposition.append,
    );
  }
}

class _NestedCatalog implements LibraryFolderCatalog {
  var parentReads = 0;

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    if (parentRelativePath.isEmpty) {
      parentReads++;
      final revision = BigInt.from(parentReads);
      return LibraryFolderPage(
        revision: revision,
        rootId: rootId,
        parentRelativePath: "",
        folders: [
          LibraryFolder(
            rootId: rootId,
            relativePath: "Album",
            name: "Album",
            directAssetCount: 1,
            descendantAssetCount: parentReads == 2 ? 1 : 2,
          ),
        ],
        nextCursor: LibraryFolderCursor(
          revision: revision,
          rootId: rootId,
          parentRelativePath: "",
          relativePath: "Album",
        ),
      );
    }
    return LibraryFolderPage(
      revision: BigInt.from(parentReads),
      rootId: rootId,
      parentRelativePath: "Album",
      folders: [
        LibraryFolder(
          rootId: rootId,
          relativePath: "Album/Child",
          name: parentReads == 1 ? "Old child" : "Current child",
          directAssetCount: 1,
          descendantAssetCount: 1,
        ),
      ],
    );
  }
}
