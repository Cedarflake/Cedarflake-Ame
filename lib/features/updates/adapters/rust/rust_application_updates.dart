import "../../../../src/rust/api/application_updates.dart" as native;
import "../../../../src/rust/domain/application_update.dart" as dto;
import "../../application/ports/application_update_service.dart";
import "../../domain/application_update.dart";

class RustApplicationUpdates implements ApplicationUpdateService {
  RustApplicationUpdates({required this.version, required this.build});
  final String? version;
  final int? build;
  Future<native.DesktopApplicationUpdates?>? _opening;

  Future<native.DesktopApplicationUpdates?> _open() => _opening ??= _create();

  Future<native.DesktopApplicationUpdates?> _create() async {
    final version = this.version;
    final build = this.build;
    if (version == null || build == null) return null;
    try {
      return await native.openApplicationUpdates(
        version: version,
        build: BigInt.from(build),
      );
    } on Object catch (error) {
      _opening = null;
      throw _failure(error);
    }
  }

  @override
  Future<ApplicationUpdateDelivery> delivery() async {
    try {
      return switch ((await _open())?.delivery()) {
        dto.ApplicationUpdateDelivery.direct =>
          ApplicationUpdateDelivery.direct,
        dto.ApplicationUpdateDelivery.microsoftStore =>
          ApplicationUpdateDelivery.microsoftStore,
        _ => ApplicationUpdateDelivery.unavailable,
      };
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  @override
  Future<void> openStore() async {
    try {
      final service = await _open();
      if (service == null) {
        throw const ApplicationUpdateFailure(
          ApplicationUpdateFailureCode.store,
          "Store delivery is unavailable",
        );
      }
      await service.openStore();
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  @override
  Future<ApplicationUpdateResult?> takeResult() async {
    try {
      final result = await (await _open())?.takeResult();
      return result == null
          ? null
          : ApplicationUpdateResult(
              ApplicationUpdateOutcome.values[result.outcome.index],
              result.code,
            );
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  @override
  Future<ApplicationUpdateDownload?> check(
    ApplicationUpdateChannel channel,
  ) async {
    try {
      final result = await (await _open())?.check(
        channel: switch (channel) {
          ApplicationUpdateChannel.stable =>
            dto.ApplicationUpdateChannel.stable,
          ApplicationUpdateChannel.preview =>
            dto.ApplicationUpdateChannel.preview,
        },
      );
      return result == null ? null : _RustDownload(result);
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  Future<void> close() async {
    (await _opening)?.dispose();
    _opening = null;
  }
}

class _RustDownload implements ApplicationUpdateDownload {
  _RustDownload(this.nativeDownload) {
    final value = nativeDownload.release();
    release = ApplicationRelease(
      version: value.version,
      build: value.build.toInt(),
      downloadBytes: value.downloadBytes.toInt(),
      notes: value.notes,
    );
  }
  final native.DesktopApplicationDownload nativeDownload;
  @override
  late final ApplicationRelease release;
  bool _closed = false;

  @override
  ApplicationDownloadProgress get progress {
    final value = nativeDownload.progress();
    return ApplicationDownloadProgress(
      phase: ApplicationDownloadPhase.values[value.phase.index],
      receivedBytes: value.receivedBytes.toInt(),
      totalBytes: value.totalBytes.toInt(),
    );
  }

  @override
  Future<void> download() async {
    try {
      nativeDownload.prepareDownload();
      await nativeDownload.download();
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  @override
  void cancel() {
    if (!_closed) nativeDownload.cancel();
  }

  @override
  Future<void> verify() => _invoke(nativeDownload.verify);
  @override
  Future<void> prepareHandoff() => _invoke(nativeDownload.prepareHandoff);
  @override
  Future<void> applyAfterExit() => _invoke(nativeDownload.applyAfterExit);
  Future<void> _invoke(Future<void> Function() operation) async {
    try {
      await operation();
    } on Object catch (error) {
      throw _failure(error);
    }
  }

  @override
  Future<void> close() async {
    if (_closed) return;
    _closed = true;
    nativeDownload.dispose();
  }
}

ApplicationUpdateFailure _failure(Object error) {
  if (error is ApplicationUpdateFailure) return error;
  if (error is dto.ApplicationUpdateError) {
    final code = switch (error.code) {
      dto.ApplicationUpdateFailureCode.store =>
        ApplicationUpdateFailureCode.store,
      dto.ApplicationUpdateFailureCode.unavailable ||
      dto.ApplicationUpdateFailureCode.cancelled =>
        ApplicationUpdateFailureCode.unavailable,
      dto.ApplicationUpdateFailureCode.network =>
        ApplicationUpdateFailureCode.network,
      dto.ApplicationUpdateFailureCode.invalidRelease =>
        ApplicationUpdateFailureCode.invalidRelease,
      dto.ApplicationUpdateFailureCode.damagedDownload =>
        ApplicationUpdateFailureCode.damagedDownload,
      dto.ApplicationUpdateFailureCode.storage =>
        ApplicationUpdateFailureCode.storage,
      dto.ApplicationUpdateFailureCode.installation =>
        ApplicationUpdateFailureCode.installation,
      dto.ApplicationUpdateFailureCode.shutdown =>
        ApplicationUpdateFailureCode.shutdown,
    };
    return ApplicationUpdateFailure(code, error.diagnostic);
  }
  return ApplicationUpdateFailure(
    ApplicationUpdateFailureCode.unavailable,
    error.toString(),
  );
}
