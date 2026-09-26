import "dart:async";

import "package:cedarflake_ame/features/library/application/library_preview_coordinator.dart";
import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("owns gallery demand and publishes compatible preview state", () async {
    final previewer = _ControlledPreviewer();
    final published = <String>[];
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 2,
      canPublish: (_) => true,
      onPublished: (asset) => published.add(asset.locationId),
    );
    addTearDown(coordinator.dispose);
    final visible = _asset("visible", LibraryPreviewStatus.pending);
    final guard = _asset("guard", LibraryPreviewStatus.pending);

    coordinator.updateGalleryDemand(
      visible: [visible],
      guard: [guard],
      previewEdges: {visible.locationId: 128, guard.locationId: 256},
    );

    expect(previewer.requests, [visible.locationId, guard.locationId]);
    expect(previewer.previewEdges, [128, 256]);
    previewer.succeed(visible.locationId, _readyAsset("visible"));
    previewer.succeed(guard.locationId, _readyAsset("guard"));
    await _flushAsyncWork();

    expect(published, [visible.locationId, guard.locationId]);
    expect(
      coordinator.resolve(visible).previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test("rejects a result before it enters the preview store", () async {
    final previewer = _ControlledPreviewer();
    final published = <String>[];
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 1,
      canPublish: (_) => false,
      onPublished: (asset) => published.add(asset.locationId),
    );
    addTearDown(coordinator.dispose);
    final pending = _asset("stale", LibraryPreviewStatus.pending);

    coordinator.updateGalleryDemand(visible: [pending]);
    previewer.succeed(pending.locationId, _readyAsset("stale"));
    await _flushAsyncWork();

    expect(published, isEmpty);
    expect(
      coordinator.resolve(pending).previewStatus,
      LibraryPreviewStatus.pending,
    );
  });

  test(
    "readmits unchanged visible demand after a rejected completion",
    () async {
      final previewer = _ControlledPreviewer();
      var canPublish = false;
      final published = <String>[];
      final coordinator = LibraryPreviewCoordinator(
        previewer: previewer,
        defaultPreviewEdge: 512,
        maxActive: 1,
        canPublish: (_) => canPublish,
        onPublished: (asset) => published.add(asset.locationId),
      );
      addTearDown(coordinator.dispose);
      final pending = _asset("returning", LibraryPreviewStatus.pending);
      coordinator.updateGalleryDemand(visible: [pending]);
      previewer.succeed(pending.locationId, _readyAsset("returning"));
      await _flushAsyncWork();
      expect(published, isEmpty);

      canPublish = true;
      coordinator.updateGalleryDemand(visible: [pending]);
      expect(previewer.requests, [pending.locationId, pending.locationId]);
      previewer.succeed(
        pending.locationId,
        _readyAsset("returning"),
        attempt: 1,
      );
      await _flushAsyncWork();
      expect(
        coordinator.resolve(pending).previewStatus,
        LibraryPreviewStatus.ready,
      );
      expect(published, [pending.locationId]);
    },
  );

  test("readmits cancelled pending demand after viewport retention", () async {
    final previewer = _ControlledPreviewer();
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 1,
      canPublish: (_) => true,
      onPublished: (_) {},
    );
    addTearDown(coordinator.dispose);
    final active = _asset("active", LibraryPreviewStatus.pending);
    final pending = _asset("pending", LibraryPreviewStatus.pending);
    coordinator.updateGalleryDemand(visible: [active, pending]);
    coordinator.retainPending([active.locationId]);
    coordinator.updateGalleryDemand(visible: [active, pending]);
    previewer.succeed(active.locationId, _readyAsset("active"));
    await _flushAsyncWork();
    expect(previewer.requests, [active.locationId, pending.locationId]);
    previewer.succeed(pending.locationId, _readyAsset("pending"));
    await _flushAsyncWork();
    expect(
      coordinator.resolve(pending).previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test(
    "retains ready pixels without repeating a failed unchanged size attempt",
    () async {
      final previewer = _ControlledPreviewer();
      final ready = _readyAsset("retained");
      final coordinator = LibraryPreviewCoordinator(
        previewer: previewer,
        defaultPreviewEdge: 512,
        maxActive: 1,
        canPublish: (_) => true,
        onPublished: (_) {},
      );
      addTearDown(coordinator.dispose);
      coordinator.updateGalleryDemand(visible: [ready]);
      previewer.fail(
        ready.locationId,
        const LibraryPreviewFailure(
          code: "preview_source_open_failed",
          message: "Locked source",
        ),
      );
      await _flushAsyncWork();
      for (var update = 0; update < 20; update++) {
        coordinator.updateGalleryDemand(visible: [ready]);
        await _flushAsyncWork();
      }
      expect(previewer.requests, [ready.locationId]);
      expect(
        coordinator.resolve(ready).previewStatus,
        LibraryPreviewStatus.ready,
      );
      final retry = coordinator.retry(ready);
      expect(previewer.requests, [ready.locationId, ready.locationId]);
      previewer.succeed(ready.locationId, ready, attempt: 1);
      expect(await retry, LibraryPreviewRequestOutcome.ready);
      coordinator.updateGalleryDemand(visible: [ready]);
      expect(previewer.requests, hasLength(2));
    },
  );

  test("does not repeat a verified viewer bucket", () async {
    final previewer = _ControlledPreviewer();
    final ready = _readyAsset("viewer");
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 1,
      canPublish: (_) => true,
      onPublished: (_) {},
    );
    addTearDown(coordinator.dispose);

    coordinator.updateGalleryDemand(
      visible: [ready],
      previewEdges: {ready.locationId: 128},
    );
    previewer.succeed(ready.locationId, ready);
    await _flushAsyncWork();

    coordinator.updateViewerDemand(ready);
    expect(previewer.previewEdges, [128, 512]);
    previewer.succeed(ready.locationId, ready, attempt: 1);
    await _flushAsyncWork();

    coordinator.updateViewerDemand(ready);
    expect(previewer.previewEdges, [128, 512]);
  });

  test(
    "changed size, source and demand can readmit a retained preview",
    () async {
      final previewer = _ControlledPreviewer();
      final coordinator = LibraryPreviewCoordinator(
        previewer: previewer,
        defaultPreviewEdge: 512,
        maxActive: 1,
        canPublish: (_) => true,
        onPublished: (_) {},
      );
      addTearDown(coordinator.dispose);
      final first = _readyAsset("changing");
      final replacement = _readyAsset("changing", sourceGeneration: BigInt.two);
      final demands = [
        (asset: first, edge: 512),
        (asset: first, edge: 256),
        (asset: replacement, edge: 256),
      ];
      for (final (attempt, demand) in demands.indexed) {
        coordinator.updateGalleryDemand(
          visible: [demand.asset],
          previewEdges: {demand.asset.locationId: demand.edge},
        );
        expect(previewer.requests, hasLength(attempt + 1));
        previewer.fail(
          demand.asset.locationId,
          const LibraryPreviewFailure(
            code: "preview_write_failed",
            message: "Storage pressure",
          ),
          attempt: attempt,
        );
        await _flushAsyncWork();
        coordinator.updateGalleryDemand(
          visible: [demand.asset],
          previewEdges: {demand.asset.locationId: demand.edge},
        );
        expect(previewer.requests, hasLength(attempt + 1));
      }
      coordinator.updateGalleryDemand();
      coordinator.updateGalleryDemand(
        visible: [replacement],
        previewEdges: {replacement.locationId: 256},
      );
      expect(previewer.previewEdges, [512, 256, 256, 256]);
      previewer.succeed(replacement.locationId, replacement, attempt: 3);
      await _flushAsyncWork();
      coordinator.updateViewerDemand(replacement);
      expect(previewer.previewEdges, [512, 256, 256, 256, 512]);
      previewer.succeed(replacement.locationId, replacement, attempt: 4);
      await _flushAsyncWork();
      coordinator.updateViewerDemand(replacement);
      expect(previewer.requests, hasLength(5));
    },
  );

  test("an offline root retains its existing automatic cooldown", () async {
    final previewer = _ControlledPreviewer();
    final ready = _readyAsset("offline-ready");
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 1,
      canPublish: (_) => true,
      onPublished: (_) {},
    );
    addTearDown(coordinator.dispose);
    coordinator.updateGalleryDemand(visible: [ready]);
    previewer.fail(
      ready.locationId,
      const LibraryPreviewFailure(
        code: "preview_root_unavailable",
        message: "Offline root",
      ),
    );
    await _flushAsyncWork();
    coordinator.updateGalleryDemand(visible: [ready]);
    expect(previewer.requests, hasLength(1));
    await Future<void>.delayed(const Duration(milliseconds: 5100));
    coordinator.updateGalleryDemand(visible: [ready]);
    expect(previewer.requests, hasLength(2));
    previewer.succeed(ready.locationId, ready, attempt: 1);
    await _flushAsyncWork();
    expect(
      coordinator.resolve(ready).previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test(
    "unchanged demand does not duplicate active, pending or ready work",
    () async {
      final previewer = _ControlledPreviewer();
      final published = <String>[];
      final coordinator = LibraryPreviewCoordinator(
        previewer: previewer,
        defaultPreviewEdge: 512,
        maxActive: 1,
        canPublish: (_) => true,
        onPublished: (asset) => published.add(asset.locationId),
      );
      addTearDown(coordinator.dispose);
      final active = _asset("active", LibraryPreviewStatus.pending);
      final pending = _asset("pending", LibraryPreviewStatus.pending);

      for (var index = 0; index < 20; index++) {
        coordinator.updateGalleryDemand(visible: [active, pending]);
      }
      expect(previewer.requests, [active.locationId]);
      previewer.succeed(active.locationId, _readyAsset("active"));
      await _flushAsyncWork();
      expect(previewer.requests, [active.locationId, pending.locationId]);
      previewer.succeed(pending.locationId, _readyAsset("pending"));
      await _flushAsyncWork();

      for (var index = 0; index < 20; index++) {
        coordinator.updateGalleryDemand(visible: [active, pending]);
      }
      await _flushAsyncWork();
      expect(previewer.requests, [active.locationId, pending.locationId]);
      expect(published, [active.locationId, pending.locationId]);
    },
  );

  for (final code in ["preview_decode_failed", "preview_root_unavailable"]) {
    test(
      "unchanged demand preserves actionable $code without retrying",
      () async {
        final previewer = _ControlledPreviewer();
        final published = <String>[];
        final coordinator = LibraryPreviewCoordinator(
          previewer: previewer,
          defaultPreviewEdge: 512,
          maxActive: 1,
          canPublish: (_) => true,
          onPublished: (asset) => published.add(asset.locationId),
        );
        addTearDown(coordinator.dispose);
        final pending = _asset("failed", LibraryPreviewStatus.pending);
        var notifications = 0;
        final subscription = coordinator.watch(pending.locationId).listen((_) {
          notifications++;
        });
        addTearDown(subscription.cancel);

        coordinator.updateGalleryDemand(visible: [pending]);
        previewer.fail(
          pending.locationId,
          LibraryPreviewFailure(code: code, message: "controlled failure"),
        );
        await _flushAsyncWork();
        for (var index = 0; index < 20; index++) {
          coordinator.updateGalleryDemand(visible: [pending]);
        }
        await _flushAsyncWork();
        expect(previewer.requests, [pending.locationId]);
        expect(published, [pending.locationId]);
        expect(notifications, 1);
        expect(
          coordinator.resolve(pending).previewStatus,
          LibraryPreviewStatus.failed,
        );
        expect(coordinator.resolve(pending).previewIssueCode, code);

        final outcome = coordinator.retry(pending);
        expect(previewer.requests, [pending.locationId, pending.locationId]);
        previewer.succeed(
          pending.locationId,
          _readyAsset("failed"),
          attempt: 1,
        );
        expect(await outcome, LibraryPreviewRequestOutcome.ready);
        expect(
          coordinator.resolve(pending).previewStatus,
          LibraryPreviewStatus.ready,
        );
      },
    );
  }

  test("reorders pending work when the gallery center changes", () async {
    final previewer = _ControlledPreviewer();
    final coordinator = LibraryPreviewCoordinator(
      previewer: previewer,
      defaultPreviewEdge: 512,
      maxActive: 1,
      canPublish: (_) => true,
      onPublished: (_) {},
    );
    addTearDown(coordinator.dispose);
    final active = _asset("active", LibraryPreviewStatus.pending);
    final upper = _asset("upper", LibraryPreviewStatus.pending);
    final center = _asset("center", LibraryPreviewStatus.pending);
    final lower = _asset("lower", LibraryPreviewStatus.pending);

    coordinator.updateGalleryDemand(visible: [active]);
    coordinator.updateGalleryDemand(visible: [active, upper, center, lower]);
    coordinator.updateGalleryDemand(visible: [active, center, upper, lower]);
    previewer.succeed(active.locationId, _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, [active.locationId, center.locationId]);
  });

  test(
    "restores one root fuse and cached failure without changing its scan",
    () async {
      final previewer = _ControlledPreviewer();
      final coordinator = LibraryPreviewCoordinator(
        previewer: previewer,
        defaultPreviewEdge: 512,
        maxActive: 1,
        canPublish: (_) => true,
        onPublished: (_) {},
      );
      addTearDown(coordinator.dispose);
      final pending = _asset("authority", LibraryPreviewStatus.pending);

      coordinator.updateGalleryDemand(visible: [pending]);
      previewer.fail(
        pending.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_identity_unproven",
          message: "root proof is unavailable",
        ),
      );
      await _flushAsyncWork();
      expect(
        coordinator.resolve(pending).previewIssueCode,
        "preview_root_identity_unproven",
      );

      coordinator.restoreRootAuthority(pending.rootId);
      expect(
        coordinator.resolve(pending).previewStatus,
        LibraryPreviewStatus.pending,
      );
      final outcome = coordinator.retry(pending);
      expect(previewer.requests, [pending.locationId, pending.locationId]);
      previewer.succeed(
        pending.locationId,
        _readyAsset("authority"),
        attempt: 1,
      );
      expect(await outcome, LibraryPreviewRequestOutcome.ready);
    },
  );
}

LibraryAsset _asset(
  String suffix,
  LibraryPreviewStatus previewStatus, {
  BigInt? sourceGeneration,
}) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: "",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    sourceRevision: const LibrarySourceRevisionEvidence(
      scheme: "windows-file-change-time-100ns-v1",
      value: "0000000000000001",
    ),
    sourceGeneration: sourceGeneration ?? BigInt.one,
    width: 320,
    height: 240,
    previewStatus: previewStatus,
  );
}

LibraryAsset _readyAsset(String suffix, {BigInt? sourceGeneration}) {
  return _asset(
    suffix,
    LibraryPreviewStatus.ready,
    sourceGeneration: sourceGeneration,
  ).withPreview(
    previewPath: "C:\\AmeCache\\$suffix.jpg",
    width: 320,
    height: 240,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

Future<void> _flushAsyncWork() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}

class _ControlledPreviewer implements LibraryPreviewer {
  final List<String> requests = [];
  final List<int> previewEdges = [];
  final Map<String, List<Completer<LibraryAsset>>> _attempts = {};

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    previewEdges.add(previewEdge);
    final completer = Completer<LibraryAsset>();
    _attempts.putIfAbsent(locationId, () => []).add(completer);
    return completer.future;
  }

  void succeed(String locationId, LibraryAsset asset, {int attempt = 0}) {
    _attempts[locationId]![attempt].complete(asset);
  }

  void fail(String locationId, Object error, {int attempt = 0}) {
    _attempts[locationId]![attempt].completeError(error);
  }
}
