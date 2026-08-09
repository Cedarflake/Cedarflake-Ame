abstract interface class LibraryPlatformActions {
  Future<void> copyText(String value);

  Future<void> revealDirectory(String path);

  Future<void> revealLibraryFolder(String rootPath, String relativePath);

  Future<void> revealFile(String path);
}
