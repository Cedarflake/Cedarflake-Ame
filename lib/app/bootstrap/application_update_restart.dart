import "../../features/updates/application/ports/application_update_service.dart";
import "../../features/updates/domain/application_update.dart";
import "../window/ame_window_retirement.dart";

class ApplicationUpdateRestart implements ApplicationUpdateRestarter {
  const ApplicationUpdateRestart(this.window);
  final AmeWindowRetirement window;

  @override
  Future<void> restart(ApplicationUpdateDownload download) async {
    try {
      await window.update(_PreparedUpdate(download));
    } on ApplicationUpdateFailure {
      rethrow;
    } on Object catch (error) {
      throw ApplicationUpdateFailure(
        ApplicationUpdateFailureCode.shutdown,
        error.toString(),
      );
    }
  }
}

class _PreparedUpdate implements AmeWindowUpdate {
  const _PreparedUpdate(this.download);
  final ApplicationUpdateDownload download;
  @override
  Future<void> prepare() => download.prepareHandoff();
  @override
  Future<void> commit() => download.applyAfterExit();
}
