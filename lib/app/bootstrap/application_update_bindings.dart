import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_riverpod/misc.dart";
import "package:shared_preferences/shared_preferences.dart";

import "../../features/updates/adapters/platform/shared_preferences_update_store.dart";
import "../../features/updates/adapters/rust/rust_application_updates.dart";
import "../../features/updates/application/application_update_controller.dart";
import "../../features/updates/application/ports/application_update_service.dart";
import "../../features/updates/domain/application_update.dart";
import "../../features/settings/presentation/widgets/updates/application_update_dialog.dart";
import "../presentation/ame_notification_actions.dart";
import "../notifications/ame_notification_controller.dart";
import "application_update_notices.dart";
import "../window/ame_build_identity.dart";

class ApplicationUpdateBindings {
  ApplicationUpdateBindings({
    required ApplicationUpdateRestarter restarter,
    AmeBuildIdentity identity = const AmeBuildIdentity.current(),
    SharedPreferencesAsync? preferences,
  }) {
    service = RustApplicationUpdates(
      version: identity.name,
      build: int.tryParse(identity.number ?? ""),
    );
    controller = ApplicationUpdateController(
      service: service,
      preferences: SharedPreferencesUpdateStore(preferences: preferences),
      restarter: restarter,
      defaultChannel: (identity.name?.contains("-") ?? true)
          ? ApplicationUpdateChannel.preview
          : ApplicationUpdateChannel.stable,
    );
  }
  late final RustApplicationUpdates service;
  late final ApplicationUpdateController controller;
  ApplicationUpdateNotices? _notices;
  List<Override> get overrides => [
    applicationUpdateControllerProvider.overrideWith((ref) {
      ref.onDispose(controller.dispose);
      return controller;
    }),
    ameNotificationActionsProvider.overrideWithValue({
      ApplicationUpdateNotices.action: (context) =>
          showApplicationUpdateDialog(context, controller),
    }),
  ];
  void attachNotifications(ProviderContainer container) {
    _notices = ApplicationUpdateNotices(
      controller,
      container.read(ameNotificationControllerProvider.notifier),
    );
  }

  Future<void> start() => controller.initialize();
  Future<void> close() async {
    _notices?.close();
    await controller.close();
    await service.close();
  }
}

final applicationUpdateRequirements = <ProviderListenable<Object?>>[
  applicationUpdateControllerProvider,
];
