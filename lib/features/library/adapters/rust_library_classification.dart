import "../../../src/rust/api/classification.dart" as native;
import "../../../src/rust/api/classification_maintenance.dart" as maintenance;
import "../application/library_classification.dart";

class RustLibraryClassification implements LibraryClassification {
  const RustLibraryClassification();

  @override
  Future<LibraryClassificationSession?> open() async {
    final session = await native.openClassificationSession();
    return session == null ? null : _RustSession(session);
  }
}

class _RustSession implements LibraryClassificationSession {
  _RustSession(this._session);
  final native.DesktopClassificationSession _session;

  @override
  Future<LibraryClassificationMaintenance?> run() async {
    final plan = await _session.run();
    return plan == null ? null : _RustMaintenance(plan);
  }

  @override
  void foregroundChanged({
    required bool readingSources,
    required bool importing,
    required bool suspended,
    required bool memoryPressure,
  }) => _session.foregroundChanged(
    readingSources: readingSources,
    importing: importing,
    suspended: suspended,
    memoryPressure: memoryPressure,
  );

  @override
  void browsingActivity() => _session.browsingActivity();

  @override
  void catalogChanged() => _session.catalogChanged();

  @override
  Future<void> rootAvailable(String rootId, BigInt generation) =>
      _session.rootAvailable(rootId: rootId, generation: generation);

  @override
  void requestClose() => _session.requestClose();

  @override
  Future<void> close() async {
    await _session.close();
    _session.dispose();
  }
}

class _RustMaintenance implements LibraryClassificationMaintenance {
  _RustMaintenance(this._plan);
  final maintenance.PreparedClassificationMaintenance _plan;

  @override
  Future<void> commit() async {
    await _plan.commit();
  }

  @override
  void cancel() => _plan.cancel();

  @override
  void dispose() => _plan.dispose();
}
