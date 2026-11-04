enum LibrarySidebarKind { albums, sourceFolders }

enum LibrarySidebarPlacement { before, after }

class LibrarySidebarMove {
  const LibrarySidebarMove({
    required this.entryId,
    required this.anchorId,
    required this.placement,
  });

  final String entryId;
  final String anchorId;
  final LibrarySidebarPlacement placement;
}
