import "../domain/library_models.dart";

abstract interface class LibrarySourceReader {
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset);
}

abstract interface class LibrarySourceReadLease {
  String get sourcePath;

  Future<void> close();
}

class LibrarySourceReadFailure implements Exception {
  const LibrarySourceReadFailure(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
