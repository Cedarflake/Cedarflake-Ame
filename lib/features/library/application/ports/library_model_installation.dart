import "../../domain/library_model_installation.dart";

abstract interface class LibraryModelInstallationSession {
  Future<void> prepare();
  LibraryModelDownloadProgress? progress();
  void cancel();
  Future<void> commit();
  Future<void> discard();
  void dispose();
}

abstract interface class LibraryModelInstallationService {
  LibraryModelInstallationSession create();
  Future<LibraryModelInstallationSession?> recover();
}
