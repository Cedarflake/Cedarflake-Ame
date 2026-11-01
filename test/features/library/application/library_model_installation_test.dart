import "dart:async";

import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("preparation completes before enablement and disposal", () async {
    final service = _Service();
    final controller = LibraryModelInstallation(service);
    final calls = <String>[];
    final run = controller.start((commit) async {
      calls.add("enable");
      expect(controller.stage, LibraryModelInstallationStage.enabling);
      expect(controller.canCancel, isFalse);
      await commit();
    });
    await Future<void>.delayed(Duration.zero);
    expect(controller.busy, isTrue);
    expect(calls, isEmpty);
    service.session.prepared.complete();
    await run;
    expect(calls, ["enable"]);
    expect(service.session.committed, isTrue);
    expect(service.session.disposed, isTrue);
    expect(controller.stage, LibraryModelInstallationStage.enabled);
    expect(controller.busy, isFalse);
    await controller.close();
    controller.dispose();
  });

  test("cancel waits for preparation retirement and never enables", () async {
    final service = _Service();
    final controller = LibraryModelInstallation(service);
    final run = controller.start(
      (_) async => fail("Cancelled installation enabled"),
    );
    await Future<void>.delayed(Duration.zero);
    controller.cancel();
    expect(controller.stage, LibraryModelInstallationStage.stopping);
    expect(controller.busy, isTrue);
    expect(service.session.disposed, isFalse);
    service.session.prepared.completeError(StateError("Transfer cancelled"));
    await run;
    expect(controller.stage, LibraryModelInstallationStage.cancelled);
    expect(service.session.committed, isFalse);
    expect(service.session.disposed, isTrue);
    await controller.close();
    controller.dispose();
  });

  test(
    "shutdown joins pending preparation even when cancel reports failure",
    () async {
      final service = _Service();
      final controller = LibraryModelInstallation(service);
      final run = controller.start(
        (_) async => fail("Closed installation enabled"),
      );
      await Future<void>.delayed(Duration.zero);
      service.session.cancelFails = true;
      var closed = false;
      final close = controller.close().whenComplete(() => closed = true);
      final closeCheck = expectLater(close, throwsStateError);
      await Future<void>.delayed(Duration.zero);
      expect(closed, isFalse);
      service.session.prepared.complete();
      await run;
      await closeCheck;
      expect(service.session.disposed, isTrue);
      expect(service.session.committed, isFalse);
      controller.dispose();
    },
  );

  test(
    "failure retires its session and explicit retry starts a new one",
    () async {
      final service = _Service();
      final controller = LibraryModelInstallation(service);
      final first = service.session;
      final run = controller.start(
        (_) async => fail("Failed preparation enabled"),
      );
      await Future<void>.delayed(Duration.zero);
      first.prepared.completeError(StateError("Offline"));
      await run;
      expect(controller.stage, LibraryModelInstallationStage.failed);
      expect(first.disposed, isTrue);
      service.session = _Session();
      final retry = controller.start((commit) => commit());
      await Future<void>.delayed(Duration.zero);
      service.session.prepared.complete();
      await retry;
      expect(controller.error, isNull);
      expect(controller.stage, LibraryModelInstallationStage.enabled);
      await controller.close();
      controller.dispose();
    },
  );
}

class _Service implements LibraryModelInstallationService {
  _Session session = _Session();
  @override
  LibraryModelInstallationSession create() => session;
}

class _Session implements LibraryModelInstallationSession {
  final prepared = Completer<void>();
  bool committed = false;
  bool disposed = false;
  bool cancelFails = false;

  @override
  Future<void> prepare() => prepared.future;
  @override
  LibraryModelDownloadProgress? progress() => null;
  @override
  void cancel() {
    if (cancelFails) throw StateError("Cancel failed");
  }

  @override
  Future<void> commit() async => committed = true;
  @override
  void dispose() => disposed = true;
}
