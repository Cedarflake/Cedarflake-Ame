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
}

Future<void> _flushAsyncWork() => Future<void>.delayed(Duration.zero);

LibraryAsset _asset(String id) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root",
    sourcePath: "C:\\Pictures\\$id.png",
    displayPath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _readyAsset(String id) {
  return _asset(id).withPreview(
    previewPath: "C:\\Cache\\$id.png",
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

class _ControlledPreviewer implements LibraryPreviewer {
  final requests = <String>[];
  final Map<String, Queue<Completer<LibraryAsset>>> _attempts = {};

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
  }) {
    requests.add(locationId);
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
