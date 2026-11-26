import "dart:async";

import "package:cedarflake_ame/app/bootstrap/application_update_notices.dart";
import "package:cedarflake_ame/app/messages/ame_message.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/updates/application/application_update_controller.dart";
import "package:cedarflake_ame/features/updates/application/application_update_state.dart";
import "package:cedarflake_ame/features/updates/application/ports/application_update_service.dart";
import "package:cedarflake_ame/features/updates/domain/application_update.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

void main() {
  late _Service service;
  late _Preferences preferences;
  late _Restarter restarter;
  late ApplicationUpdateController controller;

  setUp(() async {
    service = _Service();
    preferences = _Preferences();
    restarter = _Restarter();
    controller = ApplicationUpdateController(
      service: service,
      preferences: preferences,
      restarter: restarter,
      defaultChannel: ApplicationUpdateChannel.preview,
    );
    await controller.initialize();
  });

  tearDown(() async {
    await controller.close();
    controller.dispose();
  });

  test(
    "changing channels retires a late result without replacing the new result",
    () async {
      final oldResult = Completer<ApplicationUpdateDownload?>();
      final oldDownload = _Download();
      final newDownload = _Download();
      service.results.addAll([oldResult.future, Future.value(newDownload)]);
      final firstCheck = controller.check();
      await service.started.future;
      await controller.setPreferences(
        const ApplicationUpdatePreferences(
          channel: ApplicationUpdateChannel.stable,
          policy: ApplicationUpdatePolicy.manual,
        ),
      );
      final nextCheck = controller.check();
      expect(service.channels, [ApplicationUpdateChannel.preview]);
      oldResult.complete(oldDownload);
      await Future.wait([firstCheck, nextCheck]);

      expect(oldDownload.closed, isTrue);
      expect(newDownload.closed, isFalse);
      expect(controller.state.release, same(newDownload.release));
      expect(controller.state.phase, ApplicationUpdatePhase.available);
      expect(service.channels, [
        ApplicationUpdateChannel.preview,
        ApplicationUpdateChannel.stable,
      ]);
    },
  );

  test(
    "repeated download and automatic-policy changes retain one transfer",
    () async {
      final download = _Download();
      service.results.add(Future.value(download));
      await controller.check();
      final first = controller.download();
      final second = controller.download();
      await download.started.future;
      await controller.setPreferences(
        const ApplicationUpdatePreferences(
          channel: ApplicationUpdateChannel.preview,
          policy: ApplicationUpdatePolicy.check,
        ),
      );

      expect(download.calls, 1);
      expect(download.cancelled, isFalse);
      expect(controller.state.phase, ApplicationUpdatePhase.downloading);
      download.finish.complete();
      await Future.wait([first, second]);
      expect(controller.state.phase, ApplicationUpdatePhase.ready);
      expect(download.verifications, 0);
    },
  );

  test(
    "retirement waits for cancellation and publishes no late completion",
    () async {
      final download = _Download();
      service.results.add(Future.value(download));
      await controller.check();
      final transfer = controller.download();
      await download.started.future;
      final observed = <ApplicationUpdatePhase>[];
      controller.addListener(() => observed.add(controller.state.phase));
      var closed = false;
      final close = controller.close().then((_) => closed = true);
      expect(download.cancelled, isTrue);
      expect(closed, isFalse);
      download.finish.complete();
      await Future.wait([close, transfer]);
      expect(download.closed, isTrue);
      expect(observed, isEmpty);
    },
  );

  test("a package changed after downloading cannot trigger restart", () async {
    final download = _Download();
    service.results.add(Future.value(download));
    await controller.check();
    download.finish.complete();
    await controller.download();
    download.verifyFailure = const ApplicationUpdateFailure(
      ApplicationUpdateFailureCode.damagedDownload,
      "Package hash changed",
    );
    await controller.restart();

    expect(restarter.calls, 0);
    expect(
      controller.state.failure?.code,
      ApplicationUpdateFailureCode.damagedDownload,
    );
    expect(controller.state.phase, ApplicationUpdatePhase.failed);
  });

  test("failed preference writes leave the active channel unchanged", () async {
    preferences.failure = StateError("Preference store unavailable");
    await expectLater(
      controller.setPreferences(
        const ApplicationUpdatePreferences(
          channel: ApplicationUpdateChannel.stable,
        ),
      ),
      throwsStateError,
    );
    expect(
      controller.state.preferences.channel,
      ApplicationUpdateChannel.preview,
    );
  });

  test(
    "update feedback evolves in one entry and retires old channel actions",
    () async {
      final container = ProviderContainer();
      final notifications = container.read(
        ameNotificationControllerProvider.notifier,
      );
      final notices = ApplicationUpdateNotices(controller, notifications);
      addTearDown(() {
        notices.close();
        container.dispose();
      });
      final download = _Download();
      service.results.add(Future.value(download));
      await controller.check();
      final available = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(available.title, AmeLabel.updateAvailable);
      final transfer = controller.download();
      expect(
        container.read(ameNotificationControllerProvider).history.single.id,
        available.id,
      );
      download.finish.complete();
      await transfer;
      expect(
        container.read(ameNotificationControllerProvider).history.single.title,
        AmeLabel.updateReady,
      );
      restarter.failure = const ApplicationUpdateFailure(
        ApplicationUpdateFailureCode.shutdown,
        "Shutdown failed",
      );
      await controller.restart();
      final failure = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(failure.id, available.id);
      expect(failure.title, AmeLabel.updateFailed);
      expect(failure.severity, AmeNotificationSeverity.error);
      expect(controller.state.phase, ApplicationUpdatePhase.ready);
      await controller.setPreferences(
        const ApplicationUpdatePreferences(
          channel: ApplicationUpdateChannel.stable,
          policy: ApplicationUpdatePolicy.manual,
        ),
      );
      expect(notifications.admitAction(failure), isNull);
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isResolvedCondition,
        isTrue,
      );
    },
  );
}

class _Service implements ApplicationUpdateService {
  final results = <Future<ApplicationUpdateDownload?>>[];
  final channels = <ApplicationUpdateChannel>[];
  final started = Completer<void>();

  @override
  Future<bool> isSupported() async => true;

  @override
  Future<ApplicationUpdateResult?> takeResult() async => null;

  @override
  Future<ApplicationUpdateDownload?> check(ApplicationUpdateChannel channel) {
    channels.add(channel);
    if (!started.isCompleted) started.complete();
    return results.removeAt(0);
  }
}

class _Preferences implements ApplicationUpdatePreferenceStore {
  Object? failure;

  @override
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  }) async => ApplicationUpdatePreferences(
    channel: defaultChannel,
    policy: ApplicationUpdatePolicy.manual,
  );

  @override
  Future<void> save(ApplicationUpdatePreferences preferences) async {
    final error = failure;
    if (error != null) throw error;
  }
}

class _Restarter implements ApplicationUpdateRestarter {
  int calls = 0;
  ApplicationUpdateFailure? failure;

  @override
  Future<void> restart(ApplicationUpdateDownload download) async {
    calls++;
    final error = failure;
    if (error != null) throw error;
  }
}

class _Download implements ApplicationUpdateDownload {
  final started = Completer<void>();
  final finish = Completer<void>();
  bool cancelled = false;
  bool closed = false;
  int calls = 0;
  int verifications = 0;
  ApplicationUpdateFailure? verifyFailure;

  @override
  final release = ApplicationRelease(
    version: "0.1.0-beta.2",
    build: 5,
    downloadBytes: 1024,
    notes: "Release notes",
  );

  @override
  ApplicationDownloadProgress get progress => ApplicationDownloadProgress(
    phase: cancelled
        ? ApplicationDownloadPhase.cancelled
        : ApplicationDownloadPhase.ready,
    receivedBytes: finish.isCompleted ? 1024 : 0,
    totalBytes: 1024,
  );

  @override
  Future<void> download() async {
    calls++;
    started.complete();
    await finish.future;
  }

  @override
  void cancel() => cancelled = true;

  @override
  Future<void> verify() async {
    verifications++;
    final failure = verifyFailure;
    if (failure != null) throw failure;
  }

  @override
  Future<void> applyAfterExit() async {}

  @override
  Future<void> prepareHandoff() async {}

  @override
  Future<void> close() async {
    closed = true;
  }
}
