import "../../../../src/rust/api/classification.dart" as native;
import "../../../../src/rust/api/classification_maintenance.dart"
    as maintenance;
import "../../application/ports/library_classification.dart";
import "library_classification_failure.dart";

class RustLibraryClassification implements LibraryClassification {
  const RustLibraryClassification();

  @override
  Future<LibraryClassificationSession?> open() =>
      invokeClassificationBridge(() async {
        final session = await native.openClassificationSession();
        return session == null ? null : _RustSession(session);
      });
}

class _RustSession implements LibraryClassificationSession {
  _RustSession(this._session);
  final native.DesktopClassificationSession _session;

  @override
  Future<LibraryClassificationMaintenance?> run() =>
      invokeClassificationBridge(() async {
        final plan = await _session.run();
        return plan == null ? null : _RustMaintenance(plan);
      });

  @override
  void foregroundChanged({
    required bool readingSources,
    required bool importing,
    required bool suspended,
    required bool memoryPressure,
  }) => invokeClassificationSync(
    () => _session.foregroundChanged(
      readingSources: readingSources,
      importing: importing,
      suspended: suspended,
      memoryPressure: memoryPressure,
    ),
  );

  @override
  void browsingActivity() =>
      invokeClassificationSync(_session.browsingActivity);

  @override
  void catalogChanged() => invokeClassificationSync(_session.catalogChanged);

  @override
  Future<void> rootAvailable(String rootId, BigInt generation) =>
      invokeClassificationBridge(
        () => _session.rootAvailable(rootId: rootId, generation: generation),
      );

  @override
  void requestClose() => invokeClassificationSync(_session.requestClose);

  @override
  Future<void> close() => invokeClassificationBridge(() async {
    await _session.close();
    _session.dispose();
  });
}

class _RustMaintenance implements LibraryClassificationMaintenance {
  _RustMaintenance(this._plan);
  final maintenance.PreparedClassificationMaintenance _plan;

  @override
  Future<void> commit() => invokeClassificationBridge(_plan.commit);

  @override
  void cancel() => invokeClassificationSync(_plan.cancel);

  @override
  void dispose() => invokeClassificationSync(_plan.dispose);
}
