import "dart:async";

import "package:cedarflake_ame/features/library/application/library_preview_coordinator.dart";
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
}

LibraryAsset _asset(String suffix, LibraryPreviewStatus previewStatus) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: "",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 320,
    height: 240,
    previewStatus: previewStatus,
  );
}

LibraryAsset _readyAsset(String suffix) {
  return _asset(suffix, LibraryPreviewStatus.ready).withPreview(
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
    required int previewEdge,
    bool retry = false,
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
}
