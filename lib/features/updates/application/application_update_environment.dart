import "../domain/application_update.dart";
import "ports/application_update_service.dart";

class ApplicationUpdateEnvironment {
  const ApplicationUpdateEnvironment(
    this.delivery,
    this.preferences,
    this.result,
  );

  final ApplicationUpdateDelivery delivery;
  final ApplicationUpdatePreferences preferences;
  final ApplicationUpdateResult? result;

  static Future<ApplicationUpdateEnvironment> load({
    required ApplicationUpdateService service,
    required ApplicationUpdatePreferenceStore preferences,
    required ApplicationUpdateChannel defaultChannel,
  }) async {
    final delivery = await service.delivery();
    if (delivery != ApplicationUpdateDelivery.direct) {
      return ApplicationUpdateEnvironment(
        delivery,
        ApplicationUpdatePreferences(channel: defaultChannel),
        null,
      );
    }
    return ApplicationUpdateEnvironment(
      delivery,
      await preferences.load(defaultChannel: defaultChannel),
      await service.takeResult(),
    );
  }
}
