import "../../../src/rust/api/classification_installation.dart" as native;
import "../../../src/rust/domain/classification/installation.dart" as contract;
import "../application/library_model_installation.dart";

class RustLibraryModelInstallation implements LibraryModelInstallationService {
  const RustLibraryModelInstallation();

  @override
  LibraryModelInstallationSession create() =>
      _RustModelInstallationSession(native.createModelInstallation());

  @override
  Future<LibraryModelInstallationSession?> recover() async {
    final session = await native.recoverModelInstallation();
    return session == null ? null : _RustModelInstallationSession(session);
  }
}

class _RustModelInstallationSession implements LibraryModelInstallationSession {
  _RustModelInstallationSession(this._session);

  final native.DesktopModelInstallation _session;

  @override
  Future<void> prepare() => _session.prepare();

  @override
  LibraryModelDownloadProgress? progress() {
    final value = _session.progress();
    if (value == null) return null;
    return LibraryModelDownloadProgress(
      downloading: value.phase == contract.ModelInstallationPhase.downloading,
      completedBytes: value.completedBytes.toInt(),
      totalBytes: value.totalBytes.toInt(),
    );
  }

  @override
  void cancel() => _session.cancel();

  @override
  Future<void> commit() => _session.commit();

  @override
  Future<void> discard() => _session.discard();

  @override
  void dispose() => _session.dispose();
}
