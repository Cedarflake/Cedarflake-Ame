class LibraryConsolidatedRoot {
  const LibraryConsolidatedRoot({
    required this.previousRootId,
    required this.relativePath,
  });

  final String previousRootId;
  final String relativePath;
}

class LibraryRootConsolidation {
  const LibraryRootConsolidation({
    required this.destinationRootId,
    required this.roots,
  });

  final String destinationRootId;
  final List<LibraryConsolidatedRoot> roots;
}
