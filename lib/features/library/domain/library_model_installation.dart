enum LibraryModelInstallationStage {
  idle,
  preparing,
  verifying,
  downloading,
  stopping,
  enabling,
  enabled,
  cancelled,
  failed,
}

class LibraryModelDownloadProgress {
  const LibraryModelDownloadProgress({
    required this.downloading,
    required this.completedBytes,
    required this.totalBytes,
  });

  final bool downloading;
  final int completedBytes;
  final int totalBytes;
}
