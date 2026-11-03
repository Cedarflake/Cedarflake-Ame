abstract interface class LibraryClassification {
  Future<LibraryClassificationSession?> open();
}

abstract interface class LibraryClassificationSession {
  Future<LibraryClassificationMaintenance?> run();
  void foregroundChanged({
    required bool readingSources,
    required bool importing,
    required bool suspended,
    required bool memoryPressure,
  });
  void browsingActivity();
  void catalogChanged();
  Future<void> rootAvailable(String rootId, BigInt generation);
  void requestClose();
  Future<void> close();
}

abstract interface class LibraryClassificationMaintenance {
  Future<void> commit();
  void cancel();
  void dispose();
}
