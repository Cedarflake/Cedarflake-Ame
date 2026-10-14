import "dart:async";

import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("a retired retry cannot release the current root failure", () async {
    final previewer = _DeferredPreviewer();
    final published = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: 512,
      maxActive: 1,
      onResult: published.add,
      canPublishResult: (asset) => asset.locationId != "retired",
    );
    addTearDown(queue.dispose);
    queue.request(_asset("blocked"));
    previewer.fail("blocked", "preview_root_identity_unproven");
    await Future<void>.delayed(Duration.zero);

    LibraryPreviewRequestOutcome? retiredOutcome;
    unawaited(
      queue.retry(_asset("retired")).then((outcome) {
        retiredOutcome = outcome;
      }),
    );
    await Future<void>.delayed(Duration.zero);
    queue.request(_asset("sibling"));

    expect(retiredOutcome, LibraryPreviewRequestOutcome.superseded);
    expect(previewer.requests, ["blocked"]);
    expect(published.last.locationId, "sibling");
    expect(published.last.previewIssueCode, "preview_root_identity_unproven");
    final currentRetry = queue.retry(_asset("blocked"));
    expect(previewer.requests, ["blocked", "blocked"]);
    previewer.succeed("blocked");
    expect(await currentRetry, LibraryPreviewRequestOutcome.ready);
  });

  for (final code in [
    "preview_root_identity_unproven",
    "preview_root_identity_changed",
    "preview_root_not_found",
  ]) {
    test("availability recovery cannot automatically retry $code", () async {
      final previewer = _DeferredPreviewer();
      final published = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: published.add,
        rootUnavailableCooldown: Duration.zero,
      );
      addTearDown(queue.dispose);
      queue.request(_asset("blocked"));
      previewer.fail("blocked", code);
      await Future<void>.delayed(Duration.zero);
      queue.request(published.single);
      queue.request(_asset("sibling"));
      expect(previewer.requests, ["blocked"]);
      expect(published.last.previewIssueCode, code);
      final retry = queue.retry(published.first);
      expect(previewer.requests, ["blocked", "blocked"]);
      previewer.succeed("blocked");
      expect(await retry, LibraryPreviewRequestOutcome.ready);
    });
  }

  for (final code in [
    "preview_root_unavailable",
    "preview_root_identity_unproven",
    "preview_root_identity_changed",
    "preview_root_not_found",
  ]) {
    test("a retired source cannot spread $code to current previews", () async {
      final previewer = _DeferredPreviewer();
      final published = <LibraryAsset>[];
      var oldSourceIsCurrent = true;
      final queue = LibraryPreviewQueue(
        previewer: previewer,
        previewEdge: 512,
        maxActive: 1,
        onResult: published.add,
        canPublishResult: (asset) =>
            asset.locationId != "old" || oldSourceIsCurrent,
      );
      addTearDown(queue.dispose);

      final retired = queue.retry(_asset("old"));
      final current = queue.retry(_asset("current"));
      oldSourceIsCurrent = false;
      previewer.fail("old", code);
      await Future<void>.delayed(Duration.zero);

      expect(await retired, LibraryPreviewRequestOutcome.superseded);
      expect(published, isEmpty);
      expect(previewer.requests, ["old", "current"]);
      previewer.succeed("current");
      expect(await current, LibraryPreviewRequestOutcome.ready);
      await Future<void>.delayed(Duration.zero);

      // Later ordinary demand must not inherit a fuse from the retired source.
      queue.request(_asset("later"));
      expect(previewer.requests, ["old", "current", "later"]);
      previewer.succeed("later");
      await Future<void>.delayed(Duration.zero);
      expect(published.map((asset) => asset.locationId), ["current", "later"]);
      expect(
        published.map((asset) => asset.previewStatus),
        everyElement(LibraryPreviewStatus.ready),
      );
      expect(queue.debugRetainedGenerationCount, 0);
    });
  }
}

LibraryAsset _asset(String locationId) => LibraryAsset(
  assetId: "asset-$locationId",
  locationId: locationId,
  rootId: "same-root",
  activeScanId: "same-scan",
  sourcePath: "C:\\Generated\\$locationId.png",
  displayPath: "C:\\Generated\\$locationId.png",
  relativePath: "$locationId.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 160,
  height: 120,
  previewStatus: LibraryPreviewStatus.pending,
);

class _DeferredPreviewer implements LibraryPreviewer {
  final requests = <String>[];
  final _pending = <String, Completer<LibraryAsset>>{};

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
    final completion = Completer<LibraryAsset>();
    _pending[locationId] = completion;
    return completion.future;
  }

  void fail(String locationId, String code) {
    _pending
        .remove(locationId)!
        .completeError(
          LibraryPreviewFailure(code: code, message: "Retired source failure"),
        );
  }

  void succeed(String locationId) {
    _pending
        .remove(locationId)!
        .complete(
          _asset(locationId).withPreview(
            previewPath: "C:\\Derived\\$locationId.png",
            width: 160,
            height: 120,
            previewStatus: LibraryPreviewStatus.ready,
          ),
        );
  }
}
