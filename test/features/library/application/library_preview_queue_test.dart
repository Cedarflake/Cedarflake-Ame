import "dart:async";
import "dart:collection";

import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("bounds preview work and advances the queue", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 2,
      onResult: results.add,
    );

    queue.request(_asset("one"));
    queue.request(_asset("two"));
    queue.request(_asset("three"));

    expect(previewer.requests, ["one", "two"]);
    previewer.succeed("one", _readyAsset("one"));
    await _flushAsyncWork();

    expect(previewer.requests, ["one", "two", "three"]);
    expect(results.map((asset) => asset.locationId), ["one"]);

    previewer.succeed("two", _readyAsset("two"));
    previewer.succeed("three", _readyAsset("three"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test(
    "validates a ready preview against the requested display bucket",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );
      final ready = _readyAsset("ready");

      queue.request(ready, previewEdge: 128);
      expect(previewer.requests, isEmpty);

      queue.request(ready, previewEdge: 128, ensureSize: true);

      expect(previewer.requests, ["ready"]);
      expect(previewer.previewEdges, [128]);
      previewer.succeed("ready", ready);
      await _flushAsyncWork();
      queue.dispose();
    },
  );

  test("cancels pending work without cancelling an active decode", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.request(_asset("active"));
    queue.request(_asset("pending"));
    queue.cancel("pending");
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active"]);
    queue.dispose();
  });

  test("requires an explicit retry and publishes failure evidence", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: results.add,
    );
    final failed = _asset("failed").withPreview(
      previewPath: "",
      width: 160,
      height: 90,
      previewStatus: LibraryPreviewStatus.failed,
      previewIssueCode: "old_failure",
      previewIssueMessage: "Old failure",
    );

    queue.request(failed);
    expect(previewer.requests, isEmpty);

    queue.request(failed, retry: true);
    previewer.fail("failed", StateError("decoder stopped"));
    await _flushAsyncWork();

    expect(previewer.requests, ["failed"]);
    expect(results.single.previewStatus, LibraryPreviewStatus.failed);
    expect(results.single.previewIssueCode, "preview_request_failed");
    expect(results.single.previewIssueMessage, contains("decoder stopped"));
    queue.dispose();
  });

  test("retains only pending previews from the replacement window", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.request(_asset("active"));
    queue.request(_asset("keep"));
    queue.request(_asset("discard"));
    queue.retainPending(const ["keep"]);
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "keep"]);
    previewer.succeed("keep", _readyAsset("keep"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test(
    "starts visible work before an earlier near-direction request",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );

      queue.request(_asset("active"));
      queue.request(
        _asset("near"),
        priority: LibraryPreviewPriority.nearDirection,
      );
      queue.request(_asset("visible"));
      previewer.succeed("active", _readyAsset("active"));
      await _flushAsyncWork();

      expect(previewer.requests, ["active", "visible"]);
      previewer.succeed("visible", _readyAsset("visible"));
      await _flushAsyncWork();
      previewer.succeed("near", _readyAsset("near"));
      await _flushAsyncWork();
      queue.dispose();
    },
  );

  test(
    "drains a demand batch once after selecting its highest priority",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );

      queue.requestAll([
        (asset: _asset("near"), priority: LibraryPreviewPriority.nearDirection),
        (asset: _asset("visible"), priority: LibraryPreviewPriority.visible),
      ]);

      expect(previewer.requests, ["visible"]);
      previewer.succeed("visible", _readyAsset("visible"));
      await _flushAsyncWork();
      previewer.succeed("near", _readyAsset("near"));
      await _flushAsyncWork();
      queue.dispose();
    },
  );

  test("replaces demand before draining the new batch", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.request(_asset("active"));
    queue.request(
      _asset("near"),
      priority: LibraryPreviewPriority.nearDirection,
    );
    queue.replaceDemandAndRequestAll(
      {
        "near": LibraryPreviewPriority.guard,
        "visible": LibraryPreviewPriority.visible,
      },
      [
        (asset: _asset("near"), priority: LibraryPreviewPriority.guard),
        (asset: _asset("visible"), priority: LibraryPreviewPriority.visible),
      ],
    );
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "visible"]);
    previewer.succeed("visible", _readyAsset("visible"));
    await _flushAsyncWork();
    previewer.succeed("near", _readyAsset("near"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test("upgrades a pending location without duplicating it", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );
    final candidate = _asset("candidate");

    queue.request(_asset("active"));
    queue.request(candidate, priority: LibraryPreviewPriority.guard);
    queue.request(
      _asset("near"),
      priority: LibraryPreviewPriority.nearDirection,
    );
    queue.request(candidate, priority: LibraryPreviewPriority.viewer);
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "candidate"]);
    previewer.succeed("candidate", _readyAsset("candidate"));
    await _flushAsyncWork();
    previewer.succeed("near", _readyAsset("near"));
    await _flushAsyncWork();
    expect(
      previewer.requests.where((locationId) => locationId == "candidate"),
      hasLength(1),
    );
    queue.dispose();
  });

  test("demotes old demand before scheduling the new visible item", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.request(_asset("active"));
    queue.request(_asset("old-visible"));
    queue.updatePendingDemand({
      "old-visible": LibraryPreviewPriority.guard,
      "new-visible": LibraryPreviewPriority.visible,
    });
    queue.request(_asset("new-visible"));
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "new-visible"]);
    previewer.succeed("new-visible", _readyAsset("new-visible"));
    await _flushAsyncWork();
    previewer.succeed("old-visible", _readyAsset("old-visible"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test(
    "ignores an active result after the source generation changes",
    () async {
      final previewer = _ControlledPreviewer();
      final results = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: results.add,
      );

      queue.request(_asset("same", modifiedUnixMs: 1));
      queue.request(_asset("same", modifiedUnixMs: 2));
      previewer.succeed("same", _readyAsset("same", modifiedUnixMs: 1));
      await _flushAsyncWork();

      expect(results, isEmpty);
      expect(previewer.requests, ["same", "same"]);

      previewer.succeed("same", _readyAsset("same", modifiedUnixMs: 2));
      await _flushAsyncWork();
      expect(results.single.modifiedUnixMs, 2);
      queue.dispose();
    },
  );

  test(
    "replaces obsolete active demand without publishing its result",
    () async {
      final previewer = _ControlledPreviewer();
      final results = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: results.add,
      );

      queue.updatePendingDemand({
        "old-visible": LibraryPreviewPriority.visible,
      });
      queue.request(_asset("old-visible"));
      queue.updatePendingDemand({
        "new-visible": LibraryPreviewPriority.visible,
      });
      queue.request(_asset("new-visible"));

      expect(previewer.requests, ["old-visible", "new-visible"]);
      previewer.succeed("old-visible", _readyAsset("old-visible"));
      previewer.succeed("new-visible", _readyAsset("new-visible"));
      await _flushAsyncWork();

      expect(results.map((asset) => asset.locationId), ["new-visible"]);
      queue.dispose();
    },
  );

  test("allows only one replacement decode beyond the active limit", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 2,
      onResult: (_) {},
    );

    queue.updatePendingDemand({
      "old-one": LibraryPreviewPriority.visible,
      "old-two": LibraryPreviewPriority.visible,
    });
    queue.request(_asset("old-one"));
    queue.request(_asset("old-two"));
    queue.updatePendingDemand({
      "new-one": LibraryPreviewPriority.visible,
      "new-two": LibraryPreviewPriority.visible,
    });
    queue.request(_asset("new-one"));
    queue.request(_asset("new-two"));

    expect(previewer.requests, ["old-one", "old-two", "new-one"]);

    previewer.succeed("old-one", _readyAsset("old-one"));
    await _flushAsyncWork();
    expect(previewer.requests, ["old-one", "old-two", "new-one", "new-two"]);

    previewer.succeed("old-two", _readyAsset("old-two"));
    previewer.succeed("new-one", _readyAsset("new-one"));
    previewer.succeed("new-two", _readyAsset("new-two"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test("starts viewer demand while lower priority work is active", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.updatePendingDemand({"guard": LibraryPreviewPriority.guard});
    queue.request(_asset("guard"), priority: LibraryPreviewPriority.guard);
    queue.updatePendingDemand({
      "guard": LibraryPreviewPriority.guard,
      "viewer": LibraryPreviewPriority.viewer,
    });
    queue.request(_asset("viewer"), priority: LibraryPreviewPriority.viewer);

    expect(previewer.requests, ["guard", "viewer"]);
    expect(previewer.protectedRequests["viewer"], {"guard", "viewer"});
    previewer.succeed("viewer", _readyAsset("viewer"));
    previewer.succeed("guard", _readyAsset("guard"));
    await _flushAsyncWork();
    queue.dispose();
  });
}

Future<void> _flushAsyncWork() => Future<void>.delayed(Duration.zero);

LibraryAsset _asset(String id, {int modifiedUnixMs = 1}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root",
    sourcePath: "C:\\Pictures\\$id.png",
    displayPath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: modifiedUnixMs,
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _readyAsset(String id, {int modifiedUnixMs = 1}) {
  return _asset(id, modifiedUnixMs: modifiedUnixMs).withPreview(
    previewPath: "C:\\Cache\\$id.png",
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

class _ControlledPreviewer implements LibraryPreviewer {
  final requests = <String>[];
  final previewEdges = <int>[];
  final protectedRequests = <String, Set<String>>{};
  final Map<String, Queue<Completer<LibraryAsset>>> _attempts = {};

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    previewEdges.add(previewEdge);
    protectedRequests[locationId] = protectedLocationIds.toSet();
    final completer = Completer<LibraryAsset>();
    (_attempts[locationId] ??= Queue()).addLast(completer);
    return completer.future;
  }

  void succeed(String locationId, LibraryAsset asset) {
    _attempts[locationId]?.removeFirst().complete(asset);
  }

  void fail(String locationId, Object error) {
    _attempts[locationId]?.removeFirst().completeError(error);
  }
}
