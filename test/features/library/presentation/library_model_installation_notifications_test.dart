import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/application/ports/library_model_installation.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_model_installation_notifications.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "configuration failure does not claim a missing or partial download",
    () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final installation = LibraryModelInstallation(
        const _UnavailableConfiguration(),
      );
      addTearDown(installation.dispose);
      await installation.recover(
        (_) async => fail("Unavailable recovery enabled a model"),
      );
      final notifications = LibraryModelInstallationNotifications(
        container.read(ameNotificationControllerProvider.notifier),
      );
      notifications.accept(installation);
      final notice = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(notice.detail, "暂时无法读取模型配置，请重试。");
      expect(notice.technicalCode, "classification_installation_unavailable");
      expect(notice.actionId, "retry-model-installation");
      expect(notice.isActive, isTrue);
    },
  );
}

class _UnavailableConfiguration implements LibraryModelInstallationService {
  const _UnavailableConfiguration();

  @override
  LibraryModelInstallationSession create() => throw UnimplementedError();

  @override
  Future<LibraryModelInstallationSession?> recover() async =>
      throw const LibraryClassificationFailure(
        "classification_installation_unavailable",
        "database is locked",
      );
}
