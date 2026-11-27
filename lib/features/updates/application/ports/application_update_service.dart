import "../../domain/application_update.dart";

abstract interface class ApplicationUpdateService {
  Future<ApplicationUpdateDelivery> delivery();

  Future<void> openStore();

  Future<ApplicationUpdateResult?> takeResult();

  Future<ApplicationUpdateDownload?> check(ApplicationUpdateChannel channel);
}

abstract interface class ApplicationUpdateDownload {
  ApplicationRelease get release;

  ApplicationDownloadProgress get progress;

  Future<void> download();

  void cancel();

  Future<void> verify();

  Future<void> applyAfterExit();

  Future<void> prepareHandoff();

  Future<void> close();
}

abstract interface class ApplicationUpdatePreferenceStore {
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  });

  Future<void> save(ApplicationUpdatePreferences preferences);
}

abstract interface class ApplicationUpdateRestarter {
  Future<void> restart(ApplicationUpdateDownload download);
}
