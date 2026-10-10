import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";
import "library_catalog.dart";
import "library_controller.dart";
import "library_folder_tree.dart";

export "library_folder_tree.dart";

final libraryFolderConfiguredRootsProvider = Provider<List<LibraryRoot>>((ref) {
  return ref.watch(
    libraryControllerProvider.select((library) => library.roots),
  );
});

class LibraryFolderController extends Notifier<LibraryFolderTreeState> {
  @override
  LibraryFolderTreeState build() {
    ref.listen(libraryFolderConfiguredRootsProvider, (_, roots) {
      state = state.retainRoots(roots.map((root) => root.id).toSet());
    });
    return const LibraryFolderTreeState();
  }

  Future<void> loadBranch({
    required BigInt catalogRevision,
    required String rootId,
    required String parentRelativePath,
    bool loadMore = false,
    bool force = false,
  }) async {
    final roots = ref.read(libraryFolderConfiguredRootsProvider);
    if (!roots.any((root) => root.id == rootId)) {
      return;
    }
    final invalidationRevision = state.revision;
    if (invalidationRevision != null &&
        catalogRevision < invalidationRevision) {
      return;
    }
    state = state.invalidate(catalogRevision);
    final key = LibraryFolderBranchKey(
      rootId: rootId,
      parentRelativePath: parentRelativePath,
    );
    final current = state.branches[key] ?? const LibraryFolderBranch();
    final hasCurrentWindow =
        current.hasLoaded &&
        current.revision != null &&
        current.revision! >= catalogRevision;
    if (current.isLoading ||
        (!force &&
            !loadMore &&
            hasCurrentWindow &&
            current.errorMessage == null)) {
      return;
    }
    if (loadMore &&
        hasCurrentWindow &&
        current.nextCursor == null &&
        current.errorMessage == null) {
      return;
    }
    final after = loadMore && hasCurrentWindow ? current.nextCursor : null;

    final pending = current.copyWith(isLoading: true, errorMessage: null);
    state = state.replaceBranch(key, pending);
    try {
      final page = await ref
          .read(libraryFolderCatalogProvider)
          .loadFolderPage(
            rootId: rootId,
            parentRelativePath: parentRelativePath,
            maxItems: libraryFolderWindow,
            after: after,
          );
      if (!_ownsLoading(key, pending, catalogRevision)) {
        return;
      }
      if (page.revision < catalogRevision ||
          (current.revision != null && page.revision < current.revision!)) {
        throw const LibraryCatalogFailure(
          code: "catalog_folder_revision_changed",
          message: "图库已更新，请重新展开文件夹",
        );
      }
      if (page.rootId != rootId ||
          page.parentRelativePath != parentRelativePath) {
        throw const LibraryCatalogFailure(
          code: "catalog_folder_scope_changed",
          message: "目录范围已变化，请重新展开文件夹",
        );
      }
      final shouldAppend =
          page.disposition == LibraryFolderPageDisposition.append;
      if (shouldAppend &&
          (after == null || page.revision != current.revision)) {
        throw const LibraryCatalogFailure(
          code: "catalog_folder_page_inconsistent",
          message: "目录分页版本不一致，请重新展开文件夹",
        );
      }
      final cursor = page.nextCursor;
      if (cursor != null &&
          (cursor.revision != page.revision ||
              cursor.rootId != rootId ||
              cursor.parentRelativePath != parentRelativePath)) {
        throw const LibraryCatalogFailure(
          code: "catalog_folder_cursor_inconsistent",
          message: "目录分页范围不一致，请重新展开文件夹",
        );
      }
      state = state.publishPage(key, page);
    } on Object catch (error) {
      if (!_ownsLoading(key, pending, catalogRevision)) {
        return;
      }
      state = state.replaceBranch(
        key,
        current.copyWith(
          isLoading: false,
          hasLoaded: current.hasLoaded,
          errorMessage: error.toString(),
        ),
      );
    }
  }

  bool _ownsLoading(
    LibraryFolderBranchKey key,
    LibraryFolderBranch pending,
    BigInt revision,
  ) =>
      ref.mounted &&
      state.revision == revision &&
      identical(state.branches[key], pending);
}

final libraryFolderControllerProvider =
    NotifierProvider<LibraryFolderController, LibraryFolderTreeState>(
      LibraryFolderController.new,
    );
