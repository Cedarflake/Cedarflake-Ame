import "../../../../src/rust/api/classification_installation.dart" as native;
import "../../../../src/rust/domain/classification/installation.dart"
    as contract;
import "../../application/ports/library_model_installation.dart";
import "../../domain/library_model_installation.dart";
import "library_classification_failure.dart";

class RustLibraryModelInstallation implements LibraryModelInstallationService {
  const RustLibraryModelInstallation();

  @override
  LibraryModelInstallationSession create() => invokeClassificationSync(
    () => _RustModelInstallationSession(native.createModelInstallation()),
  );

  @override
  Future<LibraryModelInstallationSession?> recover() =>
      invokeClassificationBridge(() async {
        final session = await native.recoverModelInstallation();
        return session == null ? null : _RustModelInstallationSession(session);
      });
}

class _RustModelInstallationSession implements LibraryModelInstallationSession {
  _RustModelInstallationSession(this._session);

  final native.DesktopModelInstallation _session;

  @override
  Future<void> prepare() => invokeClassificationBridge(_session.prepare);

  @override
  LibraryModelDownloadProgress? progress() => invokeClassificationSync(() {
    final value = _session.progress();
    if (value == null) return null;
    return LibraryModelDownloadProgress(
      downloading: value.phase == contract.ModelInstallationPhase.downloading,
      completedBytes: value.completedBytes.toInt(),
      totalBytes: value.totalBytes.toInt(),
    );
  });

  @override
  void cancel() => invokeClassificationSync(_session.cancel);

  @override
  Future<void> commit() => invokeClassificationBridge(_session.commit);

  @override
  Future<void> discard() => invokeClassificationBridge(_session.discard);

  @override
  void dispose() => invokeClassificationSync(_session.dispose);
}
