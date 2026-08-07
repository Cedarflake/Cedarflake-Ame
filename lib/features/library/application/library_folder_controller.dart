import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_folder_models.dart";
import "library_catalog.dart";

class LibraryFolderBranchKey {
  const LibraryFolderBranchKey({
    required this.rootId,
    required this.parentRelativePath,
  });

  final String rootId;
  final String parentRelativePath;

  @override
  int get hashCode => Object.hash(rootId, parentRelativePath);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is LibraryFolderBranchKey &&
            rootId == other.rootId &&
            parentRelativePath == other.parentRelativePath;
  }
}

class LibraryFolderBranch {
  const LibraryFolderBranch({
    this.folders = const [],
    this.nextCursor,
    this.isLoading = false,
    this.hasLoaded = false,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final List<LibraryFolder> folders;
  final LibraryFolderCursor? nextCursor;
  final bool isLoading;
  final bool hasLoaded;
  final String? errorMessage;

  bool get hasMore => nextCursor != null;

  LibraryFolderBranch copyWith({
    List<LibraryFolder>? folders,
    Object? nextCursor = _unchanged,
    bool? isLoading,
    bool? hasLoaded,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryFolderBranch(
      folders: folders ?? this.folders,
      nextCursor: nextCursor == _unchanged
          ? this.nextCursor
          : nextCursor as LibraryFolderCursor?,
      isLoading: isLoading ?? this.isLoading,
      hasLoaded: hasLoaded ?? this.hasLoaded,
      errorMessage: errorMessage == _unchanged
          ? this.errorMessage
          : errorMessage as String?,
    );
  }
}

class LibraryFolderTreeState {
  const LibraryFolderTreeState({this.revision, this.branches = const {}});

  final BigInt? revision;
  final Map<LibraryFolderBranchKey, LibraryFolderBranch> branches;

  LibraryFolderBranch branch(String rootId, String parentRelativePath) {
    return branches[LibraryFolderBranchKey(
          rootId: rootId,
          parentRelativePath: parentRelativePath,
        )] ??
        const LibraryFolderBranch();
  }
}

class LibraryFolderController extends Notifier<LibraryFolderTreeState> {
  @override
  LibraryFolderTreeState build() => const LibraryFolderTreeState();

  Future<void> loadBranch({
    required BigInt catalogRevision,
    required String rootId,
    required String parentRelativePath,
    bool loadMore = false,
    bool force = false,
  }) async {
    _synchronizeRevision(catalogRevision);
    final key = LibraryFolderBranchKey(
      rootId: rootId,
      parentRelativePath: parentRelativePath,
    );
    final current = state.branches[key] ?? const LibraryFolderBranch();
    if (current.isLoading || (!force && !loadMore && current.hasLoaded)) {
      return;
    }
    if (loadMore && current.nextCursor == null) {
      return;
    }

    _replaceBranch(key, current.copyWith(isLoading: true, errorMessage: null));
    try {
      final page = await ref
          .read(libraryFolderCatalogProvider)
          .loadFolderPage(
            rootId: rootId,
            parentRelativePath: parentRelativePath,
            maxItems: libraryFolderWindow,
            after: loadMore ? current.nextCursor : null,
          );
      if (state.revision != catalogRevision) {
        return;
      }
      if (page.revision != catalogRevision) {
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
      final folders = loadMore
          ? _mergeFolders(current.folders, page.folders)
          : page.folders;
      _replaceBranch(
        key,
        LibraryFolderBranch(
          folders: folders,
          nextCursor: page.nextCursor,
          hasLoaded: true,
        ),
      );
    } on Object catch (error) {
      if (state.revision != catalogRevision) {
        return;
      }
      _replaceBranch(
        key,
        current.copyWith(
          isLoading: false,
          hasLoaded: current.hasLoaded,
          errorMessage: error.toString(),
        ),
      );
    }
  }

  void _synchronizeRevision(BigInt revision) {
    if (state.revision == revision) {
      return;
    }
    state = LibraryFolderTreeState(revision: revision);
  }

  void _replaceBranch(LibraryFolderBranchKey key, LibraryFolderBranch branch) {
    state = LibraryFolderTreeState(
      revision: state.revision,
      branches: Map.unmodifiable({...state.branches, key: branch}),
    );
  }

  static List<LibraryFolder> _mergeFolders(
    List<LibraryFolder> existing,
    List<LibraryFolder> next,
  ) {
    final byPath = {for (final folder in existing) folder.relativePath: folder};
    for (final folder in next) {
      byPath[folder.relativePath] = folder;
    }
    return List.unmodifiable(byPath.values);
  }
}

final libraryFolderControllerProvider =
    NotifierProvider<LibraryFolderController, LibraryFolderTreeState>(
      LibraryFolderController.new,
    );
