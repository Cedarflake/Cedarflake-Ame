import "package:cedarflake_ame/features/updates/application/application_update_controller.dart";
import "package:cedarflake_ame/features/updates/application/ports/application_update_service.dart";
import "package:cedarflake_ame/features/updates/domain/application_update.dart";
import "package:flutter_riverpod/misc.dart";

Override inertApplicationUpdates() =>
    applicationUpdateControllerProvider.overrideWith((ref) {
      final controller = ApplicationUpdateController(
        service: _Service(),
        preferences: _Preferences(),
        restarter: _Restarter(),
        defaultChannel: ApplicationUpdateChannel.preview,
      );
      ref.onDispose(controller.dispose);
      return controller;
    });

class _Service implements ApplicationUpdateService {
  @override
  Future<ApplicationUpdateDelivery> delivery() async =>
      ApplicationUpdateDelivery.unavailable;
  @override
  Future<void> openStore() =>
      throw StateError("Store navigation is not part of this fixture");
  @override
  Future<ApplicationUpdateResult?> takeResult() async => null;
  @override
  Future<ApplicationUpdateDownload?> check(ApplicationUpdateChannel channel) =>
      throw StateError("Update checks are not part of this fixture");
}

class _Preferences implements ApplicationUpdatePreferenceStore {
  @override
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  }) async => ApplicationUpdatePreferences(
    channel: defaultChannel,
    policy: ApplicationUpdatePolicy.manual,
  );
  @override
  Future<void> save(ApplicationUpdatePreferences preferences) async {}
}

class _Restarter implements ApplicationUpdateRestarter {
  @override
  Future<void> restart(ApplicationUpdateDownload download) =>
      throw StateError("Restart is not part of this fixture");
}
