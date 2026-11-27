import "dart:async";

import "package:cedarflake_ame/features/updates/application/application_update_controller.dart";
import "package:cedarflake_ame/features/updates/application/ports/application_update_service.dart";
import "package:cedarflake_ame/features/updates/domain/application_update.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("Store delivery never checks or applies direct updates", (
    tester,
  ) async {
    final service = _Store();
    final controller = _controller(service);
    await controller.initialize();
    expect(controller.state.storeManaged, isTrue);
    expect(controller.state.supported, isFalse);
    expect(controller.lastResult, isNull);
    await tester.pump(const Duration(seconds: 1));
    await controller.check();
    await controller.download();
    await controller.restart();
    await controller.setPreferences(
      const ApplicationUpdatePreferences(
        channel: ApplicationUpdateChannel.stable,
        policy: ApplicationUpdatePolicy.download,
      ),
    );
    await tester.pump(const Duration(hours: 13));
    await controller.close();
    controller.dispose();
  });

  test(
    "Store navigation is coalesced and shutdown joins the active request",
    () async {
      final service = _Store()..pending = Completer<void>();
      final controller = _controller(service);
      await controller.initialize();
      final first = controller.openStore();
      final second = controller.openStore();
      await Future<void>.delayed(Duration.zero);
      expect(service.opened, 1);
      var closed = false;
      final closing = controller.close().then((_) => closed = true);
      await Future<void>.delayed(Duration.zero);
      expect(closed, isFalse);
      service.pending!.complete();
      await Future.wait([first, second, closing]);
      controller.dispose();
    },
  );

  test(
    "failed Store navigation retains its destination and can be retried",
    () async {
      final service = _Store()..fail = true;
      final controller = _controller(service);
      await controller.openStore();
      expect(
        controller.state.failure?.code,
        ApplicationUpdateFailureCode.store,
      );
      expect(controller.state.storeManaged, isTrue);
      service.fail = false;
      await controller.openStore();
      expect(service.opened, 2);
      expect(controller.state.failure, isNull);
      await controller.close();
      controller.dispose();
    },
  );
}

ApplicationUpdateController _controller(_Store service) =>
    ApplicationUpdateController(
      service: service,
      preferences: _DirectOnlyPreferences(),
      restarter: _DirectOnlyRestarter(),
      defaultChannel: ApplicationUpdateChannel.preview,
    );

class _Store implements ApplicationUpdateService {
  int opened = 0;
  bool fail = false;
  Completer<void>? pending;

  @override
  Future<ApplicationUpdateDelivery> delivery() async =>
      ApplicationUpdateDelivery.microsoftStore;
  @override
  Future<void> openStore() async {
    opened++;
    await pending?.future;
    if (fail) {
      throw const ApplicationUpdateFailure(
        ApplicationUpdateFailureCode.store,
        "Store unavailable",
      );
    }
  }

  @override
  Future<ApplicationUpdateResult?> takeResult() =>
      throw StateError("Store must not consume MSI/portable results");
  @override
  Future<ApplicationUpdateDownload?> check(ApplicationUpdateChannel channel) =>
      throw StateError("Store must not query direct updates");
}

class _DirectOnlyPreferences implements ApplicationUpdatePreferenceStore {
  @override
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  }) => throw StateError("Store must not read direct updater preferences");
  @override
  Future<void> save(ApplicationUpdatePreferences preferences) =>
      throw StateError("Store must not save direct updater preferences");
}

class _DirectOnlyRestarter implements ApplicationUpdateRestarter {
  @override
  Future<void> restart(ApplicationUpdateDownload download) =>
      throw StateError("Store must not start a replacement helper");
}
