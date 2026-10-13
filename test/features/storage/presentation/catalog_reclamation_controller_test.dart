import "dart:async";

import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/features/storage/domain/storage_models.dart";
import "package:cedarflake_ame/features/storage/presentation/catalog_reclamation_controller.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "full storage snapshots publish usage and phase atomically without rollback",
    () {
      final controller = _controller(_Gateway());
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      final oldLoad = controller.beginStorageSnapshot();
      final terminal = _status(
        _model(
          "a",
          CatalogReclamationPhase.completed,
          fileBytes: 4096,
          liveBytes: 4096,
          reclaimableBytes: 0,
        ),
        previewUsedBytes: 256,
      );
      final observed = <StorageStatusModel?>[];
      controller.addListener(
        () => observed.add(controller.state.storageStatus),
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          terminal,
        ),
        isTrue,
      );
      expect(
        controller.acceptStorageSnapshot(
          oldLoad,
          _status(_model("a", CatalogReclamationPhase.reclaiming)),
        ),
        isFalse,
      );
      expect(observed, [terminal]);
      expect(_usage(controller.state.storageStatus!), _usage(terminal));
      expect(
        controller.state.reclamation?.phase,
        CatalogReclamationPhase.completed,
      );
    },
  );

  test(
    "a saved configuration merges independently of newer reclaimed usage",
    () async {
      final gateway = _Gateway();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      final saving = controller.beginConfigurationSave();
      final terminal = _status(
        _model(
          "a",
          CatalogReclamationPhase.completed,
          fileBytes: 4096,
          liveBytes: 4096,
          reclaimableBytes: 0,
        ),
        previewUsedBytes: 256,
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          terminal,
        ),
        isTrue,
      );
      final saved = _status(
        _model("a", CatalogReclamationPhase.reclaiming),
        previewBudgetBytes: 8192,
        configuredPreviewRoot: "new-previews",
        requiresRestart: true,
      );
      final completion = controller.completeConfigurationSave(saving, saved);
      final current = controller.state.storageStatus!;
      expect(_usage(current), _usage(terminal));
      expect(current.configuredPreviewRoot, "new-previews");
      expect(current.previewBudgetBytes, BigInt.from(8192));
      expect(current.requiresRestart, isTrue);
      expect(
        controller.state.reclamation?.phase,
        CatalogReclamationPhase.completed,
      );
      fresh.complete(_savedStatus(terminal));
      expect(await completion, isTrue);
      expect(
        await controller.completeConfigurationSave(saving, saved),
        isFalse,
      );
    },
  );

  test(
    "a load begun during save keeps newly confirmed configuration",
    () async {
      final gateway = _Gateway();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      final saving = controller.beginConfigurationSave();
      final loading = controller.beginStorageSnapshot();
      final saved = _status(
        _model("a", CatalogReclamationPhase.reclaiming),
        previewBudgetBytes: 8192,
        configuredPreviewRoot: "new-previews",
        requiresRestart: true,
      );
      final completion = controller.completeConfigurationSave(saving, saved);
      final terminal = _status(
        _model(
          "a",
          CatalogReclamationPhase.completed,
          fileBytes: 4096,
          liveBytes: 4096,
          reclaimableBytes: 0,
        ),
        previewUsedBytes: 256,
      );
      expect(controller.acceptStorageSnapshot(loading, terminal), isTrue);
      final current = controller.state.storageStatus!;
      expect(_usage(current), _usage(terminal));
      expect(current.configuredPreviewRoot, "new-previews");
      expect(current.previewBudgetBytes, BigInt.from(8192));
      expect(current.requiresRestart, isTrue);
      fresh.complete(_savedStatus(terminal));
      expect(await completion, isTrue);
    },
  );

  for (final cleanupCompletesFirst in [true, false]) {
    test(
      "saved configuration never restores a cleaned retired root (cleanup first: $cleanupCompletesFirst)",
      () async {
        final gateway = _Gateway();
        final fresh = Completer<StorageStatusModel>();
        gateway.nextStorage = fresh.future;
        final controller = _controller(gateway);
        addTearDown(controller.dispose);
        final oldStatus = _status(
          _model("a", CatalogReclamationPhase.completed),
          retiredPreviewRoots: [_retiredPreview],
        );
        expect(
          controller.acceptStorageSnapshot(
            controller.beginStorageSnapshot(),
            oldStatus,
          ),
          isTrue,
        );
        final saving = controller.beginConfigurationSave();
        final cleaning = controller.beginStorageSnapshot();
        final cleaned = _status(
          _model(
            "a",
            CatalogReclamationPhase.completed,
            fileBytes: 4096,
            liveBytes: 4096,
            reclaimableBytes: 0,
          ),
          previewUsedBytes: 256,
        );
        if (cleanupCompletesFirst) {
          expect(controller.acceptStorageSnapshot(cleaning, cleaned), isTrue);
        }
        final completion = controller.completeConfigurationSave(
          saving,
          _savedStatus(oldStatus),
        );
        if (!cleanupCompletesFirst) {
          expect(controller.acceptStorageSnapshot(cleaning, cleaned), isTrue);
        }
        var current = controller.state.storageStatus!;
        expect(current.retiredPreviewRoots, isEmpty);
        expect(_usage(current), _usage(cleaned));
        expect(current.previewBudgetBytes, BigInt.from(8192));
        expect(current.configuredPreviewRoot, "new-previews");
        fresh.complete(_savedStatus(cleaned));
        expect(await completion, isTrue);
        current = controller.state.storageStatus!;
        expect(current.retiredPreviewRoots, isEmpty);
        expect(_usage(current), _usage(cleaned));
        expect(current.previewBudgetBytes, BigInt.from(8192));
        expect(gateway.storageLoads, 1);
      },
    );
  }

  test(
    "configuration completion re-reads retired ownership instead of inferring it from paths",
    () async {
      final gateway = _Gateway();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      final oldStatus = _status(
        _model("a", CatalogReclamationPhase.completed),
        retiredPreviewRoots: [_retiredPreview],
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          oldStatus,
        ),
        isTrue,
      );
      final completion = controller.completeConfigurationSave(
        controller.beginConfigurationSave(),
        _savedStatus(oldStatus),
      );
      expect(
        controller.state.storageStatus!.configuredPreviewRoot,
        _retiredPreview.previewRoot,
      );
      expect(controller.state.storageStatus!.retiredPreviewRoots, [
        _retiredPreview,
      ]);
      expect(gateway.storageLoads, 1);
      final reconciled = _savedStatus(_status(oldStatus.catalogReclamation));
      fresh.complete(reconciled);
      expect(await completion, isTrue);
      expect(controller.state.storageStatus!.retiredPreviewRoots, isEmpty);
    },
  );

  test(
    "failed post-save refresh retries only the full snapshot and keeps saved configuration",
    () async {
      final gateway = _Gateway();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      final oldStatus = _status(
        _model("a", CatalogReclamationPhase.completed),
        retiredPreviewRoots: [_retiredPreview],
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          oldStatus,
        ),
        isTrue,
      );
      final saving = controller.beginConfigurationSave();
      final saved = _savedStatus(oldStatus);
      final completion = controller.completeConfigurationSave(saving, saved);
      fresh.completeError(StateError("controlled post-save refresh failure"));
      expect(await completion, isTrue);
      expect(
        controller.state.errorMessage,
        contains("controlled post-save refresh failure"),
      );
      expect(
        controller.state.failure?.retryTarget,
        CatalogReclamationRetryTarget.storageSnapshot,
      );
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(8192),
      );
      expect(
        controller.state.storageStatus!.configuredPreviewRoot,
        "new-previews",
      );
      expect(
        await controller.completeConfigurationSave(saving, saved),
        isFalse,
      );
      gateway.nextPoll = Future.value(oldStatus.catalogReclamation);
      await controller.refresh();
      expect(
        controller.state.failure?.retryTarget,
        CatalogReclamationRetryTarget.storageSnapshot,
      );
      expect(gateway.storageLoads, 1);
      final reconciled = _savedStatus(_status(oldStatus.catalogReclamation));
      gateway.nextStorage = Future.value(reconciled);
      await controller.retry();
      expect(gateway.storageLoads, 2);
      expect(controller.state.errorMessage, isNull);
      expect(controller.state.storageStatus!.retiredPreviewRoots, isEmpty);
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(8192),
      );
      expect(gateway.starts, 0);
    },
  );

  test(
    "a retired poll cannot release an in-flight post-save full snapshot",
    () async {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      final oldStatus = _status(
        _model("a", CatalogReclamationPhase.reclaiming),
        retiredPreviewRoots: [_retiredPreview],
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          oldStatus,
        ),
        isTrue,
      );
      final oldPoll = Completer<CatalogReclamationModel>();
      gateway.nextPoll = oldPoll.future;
      final polling = controller.refresh();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final saving = controller.completeConfigurationSave(
        controller.beginConfigurationSave(),
        _savedStatus(oldStatus),
      );
      oldPoll.complete(oldStatus.catalogReclamation);
      await polling;
      await controller.refresh();
      expect(gateway.polls, 1);
      expect(gateway.storageLoads, 1);
      final cleaned = _savedStatus(
        _status(
          _model(
            "a",
            CatalogReclamationPhase.completed,
            fileBytes: 4096,
            liveBytes: 4096,
            reclaimableBytes: 0,
          ),
          previewUsedBytes: 256,
        ),
      );
      fresh.complete(cleaned);
      expect(await saving, isTrue);
      expect(controller.state.storageStatus!.retiredPreviewRoots, isEmpty);
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(8192),
      );
      expect(_usage(controller.state.storageStatus!), _usage(cleaned));
      gateway.nextPoll = Future.value(cleaned.catalogReclamation);
      await controller.refresh();
      expect(gateway.polls, 2);
    },
  );

  for (final suspended in [false, true]) {
    test(
      "disposing settles an accepted save while its ownership read waits (suspended: $suspended)",
      () async {
        final gateway = _Gateway();
        final controller = _controller(gateway);
        final initial = _status(_model("a", CatalogReclamationPhase.cancelled));
        expect(
          controller.acceptStorageSnapshot(
            controller.beginStorageSnapshot(),
            initial,
          ),
          isTrue,
        );
        final command = Completer<CatalogReclamationModel>();
        gateway.nextStart = command.future;
        final pendingCommand = suspended ? controller.start() : null;
        final pendingStorage = Completer<StorageStatusModel>();
        gateway.nextStorage = pendingStorage.future;
        final saving = controller.completeConfigurationSave(
          controller.beginConfigurationSave(),
          _savedStatus(initial),
        );
        var notifications = 0;
        controller.addListener(() => notifications += 1);
        controller.dispose();
        expect(await saving, isTrue);
        if (suspended) {
          command.complete(_model("b", CatalogReclamationPhase.queued));
          await pendingCommand;
        } else {
          pendingStorage.complete(initial);
          await Future<void>.delayed(Duration.zero);
        }
        expect(notifications, 0);
        expect(gateway.storageLoads, suspended ? 0 : 1);
      },
    );
  }

  for (final fails in [false, true]) {
    test(
      "repeated full-snapshot retry is single-flight and reopens after settlement (fails: $fails)",
      () async {
        final gateway = _Gateway();
        final controller = _controller(gateway);
        addTearDown(controller.dispose);
        final terminal = _status(
          _model("a", CatalogReclamationPhase.completed),
        );
        expect(
          controller.acceptStorageSnapshot(
            controller.beginStorageSnapshot(),
            terminal,
          ),
          isTrue,
        );
        controller.failStorageSnapshot(
          controller.beginStorageSnapshot(),
          StateError("initial full-snapshot failure"),
        );
        final pending = Completer<StorageStatusModel>();
        gateway.nextStorage = pending.future;
        final first = controller.retry();
        final second = controller.retry();
        expect(identical(first, second), isTrue);
        expect(gateway.storageLoads, 1);
        if (fails) {
          pending.completeError(StateError("controlled retry failure"));
        } else {
          pending.complete(terminal);
        }
        await first;
        await second;
        expect(
          controller.state.errorMessage,
          fails ? contains("controlled retry failure") : isNull,
        );
        if (!fails) {
          controller.failStorageSnapshot(
            controller.beginStorageSnapshot(),
            StateError("later full-snapshot failure"),
          );
        }
        gateway.nextStorage = Future.value(terminal);
        await controller.retry();
        expect(gateway.storageLoads, 2);
        expect(controller.state.errorMessage, isNull);
      },
    );
  }

  test(
    "consecutive saves coalesce one latest full read behind an older snapshot",
    () async {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      final initial = _status(
        _model("a", CatalogReclamationPhase.completed),
        retiredPreviewRoots: [_retiredPreview],
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          initial,
        ),
        isTrue,
      );
      controller.failStorageSnapshot(
        controller.beginStorageSnapshot(),
        StateError("initial snapshot failure"),
      );
      final oldRead = Completer<StorageStatusModel>();
      gateway.nextStorage = oldRead.future;
      final retrying = controller.retry();
      final firstSave = controller.completeConfigurationSave(
        controller.beginConfigurationSave(),
        _savedStatus(initial),
      );
      final latest = _status(
        initial.catalogReclamation,
        configuredPreviewRoot: "latest-previews",
        previewBudgetBytes: 16384,
        previewUsedBytes: 128,
        requiresRestart: true,
      );
      final secondSave = controller.completeConfigurationSave(
        controller.beginConfigurationSave(),
        latest,
      );
      expect(gateway.storageLoads, 1);
      expect(
        controller.state.storageStatus!.configuredPreviewRoot,
        "latest-previews",
      );
      final freshRead = Completer<StorageStatusModel>();
      gateway.nextStorage = freshRead.future;
      oldRead.complete(initial);
      await Future<void>.delayed(Duration.zero);
      expect(gateway.storageLoads, 2);
      expect(
        controller.state.storageStatus!.configuredPreviewRoot,
        "latest-previews",
      );
      freshRead.complete(latest);
      await retrying;
      expect(await firstSave, isTrue);
      expect(await secondSave, isTrue);
      expect(controller.state.storageStatus!.retiredPreviewRoots, isEmpty);
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(16384),
      );
      expect(
        controller.state.storageStatus!.previewUsedBytes,
        BigInt.from(128),
      );
      expect(gateway.storageLoads, 2);
    },
  );

  for (final command in ["failed start", "successful start", "cancel"]) {
    test("post-save ownership refresh survives a pending $command", () async {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      final oldStatus = _status(
        _model(
          "a",
          command == "cancel"
              ? CatalogReclamationPhase.reclaiming
              : CatalogReclamationPhase.cancelled,
        ),
        retiredPreviewRoots: [_retiredPreview],
      );
      expect(
        controller.acceptStorageSnapshot(
          controller.beginStorageSnapshot(),
          oldStatus,
        ),
        isTrue,
      );
      final pendingStart = Completer<CatalogReclamationModel>();
      final pendingCancel = Completer<bool>();
      gateway.nextStart = pendingStart.future;
      gateway.nextCancel = pendingCancel.future;
      final pendingCommand = command == "cancel"
          ? controller.cancel()
          : controller.start();
      final fresh = Completer<StorageStatusModel>();
      gateway.nextStorage = fresh.future;
      final saving = controller.completeConfigurationSave(
        controller.beginConfigurationSave(),
        _savedStatus(oldStatus),
      );
      expect(gateway.storageLoads, 0);
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(8192),
      );
      if (command == "failed start") {
        pendingStart.completeError(StateError("controlled start failure"));
      } else if (command == "successful start") {
        pendingStart.complete(_model("b", CatalogReclamationPhase.queued));
      } else {
        pendingCancel.complete(true);
      }
      await pendingCommand;
      expect(gateway.storageLoads, 1);
      final currentReclamation = controller.state.reclamation!;
      fresh.complete(
        _savedStatus(_status(currentReclamation, previewUsedBytes: 256)),
      );
      expect(await saving, isTrue);
      expect(controller.state.storageStatus!.retiredPreviewRoots, isEmpty);
      expect(
        controller.state.storageStatus!.previewUsedBytes,
        BigInt.from(256),
      );
      expect(
        controller.state.storageStatus!.previewBudgetBytes,
        BigInt.from(8192),
      );
      expect(
        controller.state.reclamation?.operationId,
        command == "successful start" ? "b" : "a",
      );
      expect(gateway.storageLoads, 1);
    });
  }

  for (final fails in [false, true]) {
    test(
      "old poll cannot replace a newly started task (fails: $fails)",
      () async {
        final gateway = _Gateway();
        final controller = _controller(gateway);
        addTearDown(controller.dispose);
        _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
        final oldPoll = Completer<CatalogReclamationModel>();
        gateway.nextPoll = oldPoll.future;
        final refreshing = controller.refresh();

        _snapshot(controller, _model("a", CatalogReclamationPhase.cancelled));
        await controller.start();
        expect(controller.state.reclamation?.operationId, "b");
        if (fails) {
          oldPoll.completeError(StateError("stale operation a poll"));
        } else {
          oldPoll.complete(_model("a", CatalogReclamationPhase.completed));
        }
        await refreshing;

        expect(controller.state.reclamation?.operationId, "b");
        expect(controller.state.reclamation?.isActive, isTrue);
        expect(controller.state.errorMessage, isNull);
        expect(gateway.storageLoads, 0);
        await controller.cancel();
        expect(gateway.cancelled, ["b"]);
      },
    );
  }

  test(
    "storage snapshots from before and during start cannot overwrite it",
    () async {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.cancelled));
      final before = controller.beginStorageSnapshot();
      final startResult = Completer<CatalogReclamationModel>();
      gateway.nextStart = startResult.future;
      final starting = controller.start();
      final during = controller.beginStorageSnapshot();
      expect(controller.state.action, CatalogReclamationAction.starting);
      await controller.start();
      expect(gateway.starts, 1);
      startResult.complete(_model("b", CatalogReclamationPhase.queued));
      await starting;

      final oldStatus = _status(_model("a", CatalogReclamationPhase.completed));
      expect(controller.acceptStorageSnapshot(before, oldStatus), isFalse);
      expect(controller.acceptStorageSnapshot(during, oldStatus), isFalse);
      controller.failStorageSnapshot(before, StateError("old full load"));
      expect(controller.state.reclamation?.operationId, "b");
      expect(controller.state.errorMessage, isNull);
    },
  );

  test(
    "a newer storage snapshot wins over an earlier poll within one epoch",
    () async {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      final poll = Completer<CatalogReclamationModel>();
      gateway.nextPoll = poll.future;
      final refreshing = controller.refresh();
      _snapshot(controller, _model("a", CatalogReclamationPhase.completed));
      poll.complete(_model("a", CatalogReclamationPhase.reclaiming));
      await refreshing;
      expect(
        controller.state.reclamation?.phase,
        CatalogReclamationPhase.completed,
      );
    },
  );

  test(
    "late terminal storage refresh cannot publish over a new operation",
    () async {
      final gateway = _Gateway();
      final refreshed = <StorageStatusModel>[];
      final controller = _controller(gateway, refreshed: refreshed);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      gateway.nextPoll = Future.value(
        _model("a", CatalogReclamationPhase.cancelled),
      );
      final storage = Completer<StorageStatusModel>();
      gateway.nextStorage = storage.future;
      final refreshing = controller.refresh();
      await Future<void>.value();
      expect(gateway.storageLoads, 1);
      await controller.start();
      final currentStatus = _status(
        _model("b", CatalogReclamationPhase.queued),
      );
      gateway.nextStorage = Future.value(currentStatus);
      storage.complete(_status(_model("a", CatalogReclamationPhase.cancelled)));
      await refreshing;

      expect(controller.state.reclamation?.operationId, "b");
      expect(refreshed, [currentStatus]);
      expect(gateway.storageLoads, 2);
    },
  );

  test(
    "failed terminal storage refresh remains retryable and publishes once",
    () async {
      final gateway = _Gateway();
      final refreshed = <StorageStatusModel>[];
      final controller = _controller(gateway, refreshed: refreshed);
      addTearDown(controller.dispose);
      _snapshot(controller, _model("a", CatalogReclamationPhase.reclaiming));
      final terminal = _model("a", CatalogReclamationPhase.completed);
      gateway.nextPoll = Future.value(terminal);
      final failedStorage = Completer<StorageStatusModel>();
      gateway.nextStorage = failedStorage.future;
      final refreshing = controller.refresh();
      await Future<void>.value();
      failedStorage.completeError(StateError("controlled storage failure"));
      await refreshing;
      expect(
        controller.state.errorMessage,
        contains("controlled storage failure"),
      );
      expect(refreshed, isEmpty);

      gateway.nextStorage = Future.value(_status(terminal));
      await controller.refresh();
      expect(controller.state.errorMessage, isNull);
      expect(refreshed, hasLength(1));
      expect(gateway.storageLoads, 2);
      await controller.refresh();
      expect(gateway.storageLoads, 2);
    },
  );

  test("disposal invalidates pending polls and command completions", () async {
    for (final command in [false, true]) {
      final gateway = _Gateway();
      final controller = _controller(gateway);
      _snapshot(controller, _model("a", CatalogReclamationPhase.cancelled));
      final result = Completer<CatalogReclamationModel>();
      gateway.nextPoll = result.future;
      gateway.nextStart = result.future;
      final pending = command ? controller.start() : controller.refresh();
      var notifications = 0;
      controller.addListener(() => notifications += 1);
      controller.dispose();
      result.complete(_model("b", CatalogReclamationPhase.queued));
      await pending;
      expect(notifications, 0);
      expect(gateway.storageLoads, 0);
    }
  });
}

CatalogReclamationController _controller(
  _Gateway gateway, {
  List<StorageStatusModel>? refreshed,
}) => CatalogReclamationController(
  gateway: gateway,
  onStorageRefreshed: (status) => refreshed?.add(status),
);

void _snapshot(
  CatalogReclamationController controller,
  CatalogReclamationModel model,
) {
  expect(
    controller.acceptStorageSnapshot(
      controller.beginStorageSnapshot(),
      _status(model),
    ),
    isTrue,
  );
}

CatalogReclamationModel _model(
  String id,
  CatalogReclamationPhase phase, {
  int fileBytes = 8192,
  int liveBytes = 4096,
  int reclaimableBytes = 4096,
}) => CatalogReclamationModel(
  operationId: id,
  phase: phase,
  catalogFileBytes: BigInt.from(fileBytes),
  liveBytes: BigInt.from(liveBytes),
  reclaimableBytes: BigInt.from(reclaimableBytes),
  reclaimedBytes: BigInt.zero,
  requiredTemporaryBytes: null,
  availableTemporaryBytes: null,
  errorCode: null,
  errorMessage: null,
);

StorageStatusModel _status(
  CatalogReclamationModel model, {
  int previewUsedBytes = 512,
  int previewBudgetBytes = 1024,
  String configuredPreviewRoot = "previews",
  bool requiresRestart = false,
  List<RetiredPreviewRootModel> retiredPreviewRoots = const [],
}) => StorageStatusModel(
  settingsPath: "settings.sqlite3",
  activeCatalogPath: "catalog.sqlite3",
  activePreviewRoot: "previews",
  configuredCatalogPath: "catalog.sqlite3",
  configuredPreviewRoot: configuredPreviewRoot,
  configuredCatalogDisplayPath: "catalog.sqlite3",
  configuredPreviewDisplayPath: configuredPreviewRoot,
  previewBudgetBytes: BigInt.from(previewBudgetBytes),
  previewUsedBytes: BigInt.from(previewUsedBytes),
  catalogUsedBytes: model.catalogFileBytes,
  catalogLiveBytes: model.liveBytes,
  catalogReclaimableBytes: model.reclaimableBytes,
  catalogReclamation: model,
  requiresRestart: requiresRestart,
  retiredPreviewRoots: retiredPreviewRoots,
);

const _retiredPreview = RetiredPreviewRootModel(
  previewRoot: "new-previews",
  displayPath: "new-previews",
);

StorageStatusModel _savedStatus(StorageStatusModel usage) => _status(
  usage.catalogReclamation,
  previewUsedBytes: usage.previewUsedBytes.toInt(),
  previewBudgetBytes: 8192,
  configuredPreviewRoot: "new-previews",
  requiresRestart: true,
  retiredPreviewRoots: usage.retiredPreviewRoots,
);

List<Object> _usage(StorageStatusModel status) => [
  status.catalogUsedBytes,
  status.catalogLiveBytes,
  status.catalogReclaimableBytes,
  status.previewUsedBytes,
  status.catalogReclamation.phase,
  status.catalogReclamation.catalogFileBytes,
  status.catalogReclamation.liveBytes,
  status.catalogReclamation.reclaimableBytes,
  status.catalogReclamation.reclaimedBytes,
];

class _Gateway implements StorageSettingsGateway {
  Future<CatalogReclamationModel>? nextPoll;
  Future<CatalogReclamationModel>? nextStart;
  Future<bool>? nextCancel;
  Future<StorageStatusModel>? nextStorage;
  int starts = 0;
  int polls = 0;
  int storageLoads = 0;
  final cancelled = <String>[];

  @override
  Future<CatalogReclamationModel> loadCatalogReclamation() {
    polls += 1;
    return nextPoll ??
        Future.value(_model("b", CatalogReclamationPhase.queued));
  }

  @override
  Future<CatalogReclamationModel> startCatalogReclamation({
    required String operationId,
  }) {
    starts += 1;
    return nextStart ??
        Future.value(_model("b", CatalogReclamationPhase.queued));
  }

  @override
  Future<bool> cancelCatalogReclamation({required String operationId}) {
    cancelled.add(operationId);
    return nextCancel ?? Future.value(true);
  }

  @override
  Future<StorageStatusModel> load() {
    storageLoads += 1;
    return nextStorage ??
        Future.value(_status(_model("a", CatalogReclamationPhase.completed)));
  }

  @override
  Future<StorageStatusModel> update({
    String? catalogDirectory,
    String? previewCacheDirectory,
    required BigInt previewBudgetBytes,
  }) => throw UnimplementedError();

  @override
  Future<bool> cancelPreviewCleanup({required String operationId}) =>
      throw UnimplementedError();

  @override
  Stream<PreviewCleanupUpdate> clearPreviews({required String operationId}) =>
      throw UnimplementedError();

  @override
  Stream<PreviewCleanupUpdate> clearRetiredPreviews({
    required String previewRoot,
    required String operationId,
  }) => throw UnimplementedError();
}
