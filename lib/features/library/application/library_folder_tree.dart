import "../domain/library_folder_models.dart";

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
    this.revision,
    this.folders = const [],
    this.nextCursor,
    this.isLoading = false,
    this.hasLoaded = false,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final BigInt? revision;
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
      revision: revision,
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

  LibraryFolderTreeState invalidate(BigInt nextRevision) {
    if (revision == nextRevision) {
      return this;
    }
    // Published windows remain visible, but no earlier loading slot survives.
    return LibraryFolderTreeState(
      revision: nextRevision,
      branches: Map.unmodifiable({
        for (final entry in branches.entries)
          entry.key: entry.value.copyWith(isLoading: false),
      }),
    );
  }

  LibraryFolderTreeState retainRoots(Set<String> rootIds) {
    if (branches.keys.every((key) => rootIds.contains(key.rootId))) {
      return this;
    }
    return LibraryFolderTreeState(
      revision: revision,
      branches: Map.unmodifiable({
        for (final entry in branches.entries)
          if (rootIds.contains(entry.key.rootId)) entry.key: entry.value,
      }),
    );
  }

  LibraryFolderTreeState replaceBranch(
    LibraryFolderBranchKey key,
    LibraryFolderBranch branch,
  ) => LibraryFolderTreeState(
    revision: revision,
    branches: Map.unmodifiable({...branches, key: branch}),
  );

  LibraryFolderTreeState publishPage(
    LibraryFolderBranchKey key,
    LibraryFolderPage page,
  ) {
    final folders = page.disposition == LibraryFolderPageDisposition.append
        ? _mergeFolders(branches[key]?.folders ?? const [], page.folders)
        : page.folders;
    final branch = LibraryFolderBranch(
      revision: page.revision,
      folders: folders,
      nextCursor: page.nextCursor,
      hasLoaded: true,
    );
    final children = {
      for (final folder in branch.folders) folder.relativePath: folder,
    };
    final prefix = key.parentRelativePath.isEmpty
        ? ""
        : "${key.parentRelativePath}/";
    final retained = {...branches, key: branch};
    retained.removeWhere((candidate, snapshot) {
      if (candidate == key ||
          candidate.rootId != key.rootId ||
          !candidate.parentRelativePath.startsWith(prefix)) {
        return false;
      }
      // A slower parent read cannot disprove a newer child's published window.
      if (snapshot.revision != null && snapshot.revision! > page.revision) {
        return false;
      }
      final suffix = candidate.parentRelativePath.substring(prefix.length);
      final childPath = "$prefix${suffix.split('/').first}";
      final child = children[childPath];
      if (child != null) {
        return !child.hasChildFolders;
      }
      // An unfinished first window cannot prove another child was removed.
      return !branch.hasMore;
    });
    return LibraryFolderTreeState(
      revision: revision,
      branches: Map.unmodifiable(retained),
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

  LibraryFolderBranch branch(String rootId, String parentRelativePath) {
    return branches[LibraryFolderBranchKey(
          rootId: rootId,
          parentRelativePath: parentRelativePath,
        )] ??
        const LibraryFolderBranch();
  }
}
