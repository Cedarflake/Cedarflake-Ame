import "dart:async";
import "dart:collection";

import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "preserves typed source failures and accepts a refreshed generation",
    () async {
      final previewer = _ControlledPreviewer();
      final results = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: results.add,
      );
      final outcome = queue.retry(_asset("changed"));
      previewer.fail(
        "changed",
        const LibraryPreviewFailure(
          code: "source_revision_changed_during_scan",
          message: "The source revision changed",
        ),
      );
      expect(await outcome, LibraryPreviewRequestOutcome.failed);
      expect(
        results.single.previewIssueCode,
        "source_revision_changed_during_scan",
      );
      expect(results.single.previewIssueMessage, "The source revision changed");
      queue.request(results.single);
      expect(previewer.requests, ["changed"]);
      final refreshed = _asset("changed", sourceGeneration: BigInt.from(2));
      queue.request(refreshed);
      previewer.succeed(
        "changed",
        _readyAsset("changed", sourceGeneration: BigInt.from(2)),
      );
      await _flushAsyncWork();
      expect(results.last.previewStatus, LibraryPreviewStatus.ready);
      expect(results.last.sourceGeneration, BigInt.from(2));
      queue.dispose();
    },
  );

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
    final pendingOutcome = queue.retry(_asset("pending"));
    queue.cancel("pending");
    expect(await pendingOutcome, LibraryPreviewRequestOutcome.cancelled);
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

    final retryOutcome = queue.retry(failed);
    previewer.fail("failed", StateError("decoder stopped"));
    await _flushAsyncWork();

    expect(previewer.requests, ["failed"]);
    expect(results.single.previewStatus, LibraryPreviewStatus.failed);
    expect(results.single.previewIssueCode, "preview_request_failed");
    expect(results.single.previewIssueMessage, contains("decoder stopped"));
    expect(await retryOutcome, LibraryPreviewRequestOutcome.failed);
    queue.dispose();
  });

  test(
    "propagates exact source context and forces an explicit retry",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );
      const revision = LibrarySourceRevisionEvidence(
        scheme: "windows-file-change-time-100ns-v1",
        value: "0000000000000007",
      );
      final asset = _asset(
        "context",
        activeScanId: "scan-7",
        sourceRevision: revision,
        sourceGeneration: BigInt.from(7),
      );

      final outcome = queue.retry(asset);

      expect(previewer.forceRequests, [true]);
      expect(previewer.sourceContexts, [
        (
          rootId: "root",
          scanId: "scan-7",
          revision: revision,
          generation: BigInt.from(7),
        ),
      ]);
      previewer.succeed(
        "context",
        _readyAsset(
          "context",
          activeScanId: "scan-7",
          sourceRevision: revision,
          sourceGeneration: BigInt.from(7),
        ),
      );
      expect(await outcome, LibraryPreviewRequestOutcome.ready);
      queue.dispose();
    },
  );

  test("does not publish a superseded backend response as failed", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: results.add,
    );

    final outcome = queue.retry(_asset("superseded"));
    previewer.fail(
      "superseded",
      const LibraryPreviewFailure(
        code: "preview_request_superseded",
        message: "source context changed",
      ),
    );

    expect(await outcome, LibraryPreviewRequestOutcome.superseded);
    expect(results, isEmpty);
    queue.dispose();
  });

  test("replaces a compatible normal pending request with a retry", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: results.add,
    );
    final pending = _asset("pending");

    queue.request(_asset("active"));
    queue.request(pending);
    final retryOutcome = queue.retry(pending);

    expect(previewer.requests, ["active"]);
    expect(previewer.forceRequests, [false]);
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "pending"]);
    expect(previewer.forceRequests, [false, true]);
    expect(previewer.maxConcurrentRequests, 1);
    previewer.succeed("pending", _readyAsset("pending"));

    expect(await retryOutcome, LibraryPreviewRequestOutcome.ready);
    expect(results.map((asset) => asset.locationId), ["active", "pending"]);
    queue.dispose();
  });

  test("queues a retry behind a compatible normal active request", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: results.add,
    );
    final active = _asset("active");

    queue.request(active);
    final retryOutcome = queue.retry(active);

    expect(previewer.requests, ["active"]);
    expect(previewer.forceRequests, [false]);
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(results, isEmpty);
    expect(previewer.requests, ["active", "active"]);
    expect(previewer.forceRequests, [false, true]);
    expect(previewer.maxConcurrentRequests, 1);
    previewer.succeed("active", _readyAsset("active"));

    expect(await retryOutcome, LibraryPreviewRequestOutcome.ready);
    expect(results.map((asset) => asset.locationId), ["active"]);
    queue.dispose();
  });

  test(
    "shares a compatible active request with explicit retry waiters",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );
      final asset = _asset("shared");

      queue.request(asset, retry: true);
      final firstOutcome = queue.retry(asset);
      final secondOutcome = queue.retry(asset);

      expect(previewer.requests, ["shared"]);
      previewer.succeed("shared", _readyAsset("shared"));
      expect(await firstOutcome, LibraryPreviewRequestOutcome.ready);
      expect(await secondOutcome, LibraryPreviewRequestOutcome.ready);
      queue.dispose();
    },
  );

  test("ends an explicit retry when a newer source supersedes it", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );
    final first = _asset("same");
    final replacement = _asset("same", modifiedUnixMs: 2);

    final firstOutcome = queue.retry(first);
    final replacementOutcome = queue.retry(replacement);

    expect(await firstOutcome, LibraryPreviewRequestOutcome.superseded);
    previewer.succeed("same", _readyAsset("same"));
    await _flushAsyncWork();
    expect(previewer.requests, ["same", "same"]);
    previewer.succeed("same", _readyAsset("same", modifiedUnixMs: 2));
    expect(await replacementOutcome, LibraryPreviewRequestOutcome.ready);
    queue.dispose();
  });

  test("ends active explicit retries on context invalidation", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    final outcome = queue.retry(_asset("invalidated"));
    queue.invalidateAll();

    expect(await outcome, LibraryPreviewRequestOutcome.contextInvalidated);
    previewer.succeed("invalidated", _readyAsset("invalidated"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test("ends active explicit retries when the queue is disposed", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    final outcome = queue.retry(_asset("disposed"));
    queue.dispose();

    expect(await outcome, LibraryPreviewRequestOutcome.disposed);
    previewer.succeed("disposed", _readyAsset("disposed"));
    await _flushAsyncWork();
  });

  test("ends a retry when the publication owner rejects its result", () async {
    final previewer = _ControlledPreviewer();
    final results = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: results.add,
      canPublishResult: (_) => false,
    );

    final outcome = queue.retry(_asset("rejected"));
    previewer.succeed("rejected", _readyAsset("rejected"));

    expect(await outcome, LibraryPreviewRequestOutcome.superseded);
    expect(results, isEmpty);
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

  test("uses the latest center-out rank within one demand priority", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );

    queue.request(_asset("active"));
    queue.request(_asset("upper"));
    queue.request(_asset("center"));
    queue.request(_asset("lower"));
    queue.replaceDemandAndRequestAll(
      {
        "center": LibraryPreviewPriority.visible,
        "upper": LibraryPreviewPriority.visible,
        "lower": LibraryPreviewPriority.visible,
      },
      [
        (asset: _asset("center"), priority: LibraryPreviewPriority.visible),
        (asset: _asset("upper"), priority: LibraryPreviewPriority.visible),
        (asset: _asset("lower"), priority: LibraryPreviewPriority.visible),
      ],
    );
    previewer.succeed("active", _readyAsset("active"));
    await _flushAsyncWork();

    expect(previewer.requests, ["active", "center"]);
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

      queue.request(_asset("same", sourceGeneration: BigInt.one));
      queue.request(_asset("same", sourceGeneration: BigInt.two));
      previewer.succeed(
        "same",
        _readyAsset("same", sourceGeneration: BigInt.one),
      );
      await _flushAsyncWork();

      expect(results, isEmpty);
      expect(previewer.requests, ["same", "same"]);

      previewer.succeed(
        "same",
        _readyAsset("same", sourceGeneration: BigInt.two),
      );
      await _flushAsyncWork();
      expect(results.single.sourceGeneration, BigInt.two);
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

      expect(previewer.requests, ["old-visible"]);
      previewer.succeed("old-visible", _readyAsset("old-visible"));
      await _flushAsyncWork();
      expect(previewer.requests, ["old-visible", "new-visible"]);
      previewer.succeed("new-visible", _readyAsset("new-visible"));
      await _flushAsyncWork();

      expect(results.map((asset) => asset.locationId), ["new-visible"]);
      queue.dispose();
    },
  );

  test(
    "never exceeds the hard limit while replacing obsolete demand",
    () async {
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

      expect(previewer.requests, ["old-one", "old-two"]);

      previewer.succeed("old-one", _readyAsset("old-one"));
      await _flushAsyncWork();
      expect(previewer.requests, ["old-one", "old-two", "new-one"]);

      previewer.succeed("old-two", _readyAsset("old-two"));
      await _flushAsyncWork();
      expect(previewer.requests, ["old-one", "old-two", "new-one", "new-two"]);

      previewer.succeed("new-one", _readyAsset("new-one"));
      previewer.succeed("new-two", _readyAsset("new-two"));
      await _flushAsyncWork();
      queue.dispose();
    },
  );

  test("queues viewer demand without exceeding the hard limit", () async {
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

    expect(previewer.requests, ["guard"]);
    previewer.succeed("guard", _readyAsset("guard"));
    await _flushAsyncWork();
    expect(previewer.requests, ["guard", "viewer"]);
    expect(previewer.protectedRequests["viewer"], {"guard", "viewer"});
    previewer.succeed("viewer", _readyAsset("viewer"));
    await _flushAsyncWork();
    queue.dispose();
  });

  test(
    "fuses one unproven root context while another root keeps draining",
    () async {
      final previewer = _ControlledPreviewer();
      final results = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: results.add,
      );
      final first = _asset("first");
      final sameRoot = _asset("same-root");
      final otherRoot = _asset("other-root", rootId: "root-2");

      queue.request(first);
      final blockedWaiter = queue.retry(sameRoot);
      queue.request(otherRoot);
      previewer.fail(
        first.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_identity_unproven",
          message: "root proof is unavailable",
        ),
      );
      await _flushAsyncWork();

      expect(await blockedWaiter, LibraryPreviewRequestOutcome.updateRequired);
      expect(previewer.requests, [first.locationId, otherRoot.locationId]);
      expect(results.map((asset) => asset.locationId), [
        first.locationId,
        sameRoot.locationId,
      ]);
      expect(
        results.map((asset) => asset.previewIssueCode),
        everyElement("preview_root_identity_unproven"),
      );

      final immediateRetry = queue.retry(sameRoot);
      expect(previewer.requests, [first.locationId, otherRoot.locationId]);

      previewer.succeed(
        otherRoot.locationId,
        _readyAsset("other-root", rootId: "root-2"),
      );
      await _flushAsyncWork();
      expect(previewer.requests, [
        first.locationId,
        otherRoot.locationId,
        sameRoot.locationId,
      ]);
      previewer.fail(
        sameRoot.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_identity_unproven",
          message: "root proof is still unavailable",
        ),
      );
      expect(await immediateRetry, LibraryPreviewRequestOutcome.updateRequired);
      await _flushAsyncWork();
      expect(queue.debugRetainedGenerationCount, 0);
      queue.dispose();
    },
  );

  test(
    "cools down one unavailable root without starving another root",
    () async {
      final previewer = _ControlledPreviewer();
      final results = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: results.add,
        rootUnavailableCooldown: const Duration(milliseconds: 10),
      );
      final unavailable = _asset("unavailable");
      final sameRoot = _asset("same-root-unavailable");
      final otherRoot = _asset("available", rootId: "root-2");

      queue.request(unavailable);
      final blockedWaiter = queue.retry(sameRoot);
      queue.request(otherRoot);
      previewer.fail(
        unavailable.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_unavailable",
          message: "the source volume is offline",
        ),
      );
      await _flushAsyncWork();

      expect(await blockedWaiter, LibraryPreviewRequestOutcome.failed);
      expect(previewer.requests, [
        unavailable.locationId,
        otherRoot.locationId,
      ]);
      expect(
        results
            .where((asset) => asset.rootId == unavailable.rootId)
            .map((asset) => asset.previewIssueCode),
        everyElement("preview_root_unavailable"),
      );

      queue.request(_asset("during-cooldown"));
      expect(previewer.requests, [
        unavailable.locationId,
        otherRoot.locationId,
      ]);
      expect(results.last.previewIssueCode, "preview_root_unavailable");

      previewer.succeed(
        otherRoot.locationId,
        _readyAsset("available", rootId: "root-2"),
      );
      await Future<void>.delayed(const Duration(milliseconds: 20));
      final afterCooldown = _asset("after-cooldown");
      queue.request(afterCooldown);
      expect(previewer.requests, [
        unavailable.locationId,
        otherRoot.locationId,
        afterCooldown.locationId,
      ]);
      previewer.fail(
        afterCooldown.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_unavailable",
          message: "the source volume is still offline",
        ),
      );
      await _flushAsyncWork();
      expect(results.last.previewIssueCode, "preview_root_unavailable");
      expect(queue.debugRetainedGenerationCount, 0);
      queue.dispose();
    },
  );

  test("a new catalog scan generation releases a root fuse", () async {
    final previewer = _ControlledPreviewer();
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: (_) {},
    );
    final oldContext = _asset("old-scan", activeScanId: "scan-old");

    queue.request(oldContext);
    previewer.fail(
      oldContext.locationId,
      const LibraryPreviewFailure(
        code: "preview_root_identity_changed",
        message: "root identity changed",
      ),
    );
    await _flushAsyncWork();

    final newContext = _asset("new-scan", activeScanId: "scan-new");
    queue.request(newContext);
    expect(previewer.requests, [oldContext.locationId, newContext.locationId]);
    previewer.succeed(
      newContext.locationId,
      _readyAsset("new-scan", activeScanId: "scan-new"),
    );
    await _flushAsyncWork();
    expect(queue.debugRetainedGenerationCount, 0);
    queue.dispose();
  });

  test(
    "an authority restoration releases the root fuse without a new scan",
    () async {
      final previewer = _ControlledPreviewer();
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: (_) {},
      );
      final oldContext = _asset("old-context", activeScanId: "scan-old");

      queue.request(oldContext);
      previewer.fail(
        oldContext.locationId,
        const LibraryPreviewFailure(
          code: "preview_root_identity_unproven",
          message: "root proof is unavailable",
        ),
      );
      await _flushAsyncWork();

      queue.clearBlockedRoot(oldContext.rootId);
      final restoredContext = _asset(
        "restored-context",
        activeScanId: "scan-old",
      );
      final outcome = queue.retry(restoredContext);
      expect(previewer.requests, [
        oldContext.locationId,
        restoredContext.locationId,
      ]);
      previewer.succeed(
        restoredContext.locationId,
        _readyAsset("restored-context", activeScanId: "scan-old"),
      );
      expect(await outcome, LibraryPreviewRequestOutcome.ready);
      await _flushAsyncWork();
      expect(queue.debugRetainedGenerationCount, 0);
      queue.dispose();
    },
  );
}

Future<void> _flushAsyncWork() => Future<void>.delayed(Duration.zero);

LibraryAsset _asset(
  String id, {
  int modifiedUnixMs = 1,
  String rootId = "root",
  String activeScanId = "scan-1",
  LibrarySourceRevisionEvidence? sourceRevision =
      const LibrarySourceRevisionEvidence(
        scheme: "windows-file-change-time-100ns-v1",
        value: "0000000000000001",
      ),
  BigInt? sourceGeneration,
}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: rootId,
    activeScanId: activeScanId,
    sourcePath: "C:\\Pictures\\$id.png",
    displayPath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: modifiedUnixMs,
    sourceRevision: sourceRevision,
    sourceGeneration: sourceGeneration ?? BigInt.one,
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _readyAsset(
  String id, {
  int modifiedUnixMs = 1,
  String rootId = "root",
  String activeScanId = "scan-1",
  LibrarySourceRevisionEvidence? sourceRevision =
      const LibrarySourceRevisionEvidence(
        scheme: "windows-file-change-time-100ns-v1",
        value: "0000000000000001",
      ),
  BigInt? sourceGeneration,
}) {
  return _asset(
    id,
    modifiedUnixMs: modifiedUnixMs,
    rootId: rootId,
    activeScanId: activeScanId,
    sourceRevision: sourceRevision,
    sourceGeneration: sourceGeneration,
  ).withPreview(
    previewPath: "C:\\Cache\\$id.png",
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

class _ControlledPreviewer implements LibraryPreviewer {
  final requests = <String>[];
  final forceRequests = <bool>[];
  final sourceContexts =
      <
        ({
          String rootId,
          String scanId,
          LibrarySourceRevisionEvidence? revision,
          BigInt generation,
        })
      >[];
  final previewEdges = <int>[];
  final protectedRequests = <String, Set<String>>{};
  final Map<String, Queue<Completer<LibraryAsset>>> _attempts = {};
  int concurrentRequests = 0;
  int maxConcurrentRequests = 0;

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
    forceRequests.add(force);
    sourceContexts.add((
      rootId: expectedRootId,
      scanId: expectedScanId,
      revision: expectedSourceRevision,
      generation: expectedSourceGeneration,
    ));
    previewEdges.add(previewEdge);
    protectedRequests[locationId] = protectedLocationIds.toSet();
    concurrentRequests++;
    if (concurrentRequests > maxConcurrentRequests) {
      maxConcurrentRequests = concurrentRequests;
    }
    final completer = Completer<LibraryAsset>();
    (_attempts[locationId] ??= Queue()).addLast(completer);
    return completer.future;
  }

  void succeed(String locationId, LibraryAsset asset) {
    final completer = _attempts[locationId]?.removeFirst();
    if (completer == null) {
      return;
    }
    concurrentRequests--;
    completer.complete(asset);
  }

  void fail(String locationId, Object error) {
    final completer = _attempts[locationId]?.removeFirst();
    if (completer == null) {
      return;
    }
    concurrentRequests--;
    completer.completeError(error);
  }
}
