enum LibraryScrollSectionKind { name, newArrivals, earlier }

class LibraryScrollSection {
  const LibraryScrollSection({
    required this.kind,
    required this.key,
    required this.startOrdinal,
    required this.itemCount,
  });

  final LibraryScrollSectionKind kind;
  final String key;
  final int startOrdinal;
  final int itemCount;
}
