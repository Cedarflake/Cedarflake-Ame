abstract interface class LibraryForegroundReadActivity {
  LibraryForegroundReadLease acquire();
}

abstract interface class LibraryForegroundReadLease {
  void close();
}
