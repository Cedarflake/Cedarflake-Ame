import "library_models.dart";

sealed class LibraryQueryActivity {
  const LibraryQueryActivity();
}

class LibraryQueryIdle extends LibraryQueryActivity {
  const LibraryQueryIdle();
}

class LibraryQueryLoading extends LibraryQueryActivity {
  const LibraryQueryLoading(this.requestedQuery);

  final LibraryGalleryQuery requestedQuery;
}

class LibraryQueryFailed extends LibraryQueryActivity {
  const LibraryQueryFailed({
    required this.requestedQuery,
    required this.message,
  });

  final LibraryGalleryQuery requestedQuery;
  final String message;
}
