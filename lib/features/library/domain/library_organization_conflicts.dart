class LibraryConflictAlbum {
  const LibraryConflictAlbum({
    required this.id,
    required this.name,
    required this.retired,
  });
  final String id;
  final String name;
  final bool retired;
}

class LibraryConflictAlbums {
  const LibraryConflictAlbums({
    required this.albums,
    required this.revision,
    this.nextAlbumId,
  });
  final List<LibraryConflictAlbum> albums;
  final BigInt revision;
  final String? nextAlbumId;
}

class LibraryConflictStatus {
  const LibraryConflictStatus({required this.revision, required this.groups});
  final BigInt revision;
  final int groups;
}
