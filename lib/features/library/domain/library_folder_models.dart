class LibraryFolder {
  const LibraryFolder({
    required this.rootId,
    required this.relativePath,
    required this.name,
    required this.directAssetCount,
    required this.descendantAssetCount,
  });

  final String rootId;
  final String relativePath;
  final String name;
  final int directAssetCount;
  final int descendantAssetCount;

  bool get hasChildFolders => descendantAssetCount > directAssetCount;
}

class LibraryFolderCursor {
  const LibraryFolderCursor({
    required this.revision,
    required this.rootId,
    required this.parentRelativePath,
    required this.relativePath,
  });

  final BigInt revision;
  final String rootId;
  final String parentRelativePath;
  final String relativePath;
}

class LibraryFolderPage {
  const LibraryFolderPage({
    required this.revision,
    required this.rootId,
    required this.parentRelativePath,
    required this.folders,
    this.nextCursor,
  });

  final BigInt revision;
  final String rootId;
  final String parentRelativePath;
  final List<LibraryFolder> folders;
  final LibraryFolderCursor? nextCursor;
}
