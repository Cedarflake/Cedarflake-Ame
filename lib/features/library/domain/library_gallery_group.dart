enum LibraryGalleryGroupFilter { unrestricted, exact, similar }

sealed class LibraryGalleryGroupKey {
  const LibraryGalleryGroupKey();

  String get entryId;

  static LibraryGalleryGroupKey? fromEntryId(String entryId) {
    if (entryId.startsWith("similar:")) {
      return LibrarySimilarGalleryGroupKey(entryId.substring(8));
    }
    if (entryId.startsWith("exact:")) {
      final separator = entryId.lastIndexOf(":");
      final bytes = BigInt.tryParse(entryId.substring(separator + 1));
      if (separator > 6 && bytes != null) {
        return LibraryExactGalleryGroupKey(
          entryId.substring(6, separator),
          bytes,
        );
      }
    }
    return null;
  }
}

class LibraryExactGalleryGroupKey extends LibraryGalleryGroupKey {
  const LibraryExactGalleryGroupKey(this.digest, this.byteLength);

  final String digest;
  final BigInt byteLength;

  @override
  String get entryId => "exact:$digest:$byteLength";

  @override
  bool operator ==(Object other) =>
      other is LibraryExactGalleryGroupKey &&
      digest == other.digest &&
      byteLength == other.byteLength;

  @override
  int get hashCode => Object.hash(digest, byteLength);
}

class LibrarySimilarGalleryGroupKey extends LibraryGalleryGroupKey {
  const LibrarySimilarGalleryGroupKey(this.groupId);

  final String groupId;

  @override
  String get entryId => "similar:$groupId";

  @override
  bool operator ==(Object other) =>
      other is LibrarySimilarGalleryGroupKey && groupId == other.groupId;

  @override
  int get hashCode => groupId.hashCode;
}

class LibraryGalleryGroup {
  const LibraryGalleryGroup({required this.key, required this.memberCount});

  final LibraryGalleryGroupKey key;
  final int memberCount;
}
