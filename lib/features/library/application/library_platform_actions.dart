abstract interface class LibraryPlatformActions {
  Future<void> copyText(String value);

  Future<void> openDirectory(String path);

  Future<void> openLibraryFolder(String rootPath, String relativePath);

  Future<void> revealFile(String path);
}
