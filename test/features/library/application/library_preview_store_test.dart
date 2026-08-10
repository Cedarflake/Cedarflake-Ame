import "package:cedarflake_ame/features/library/application/library_preview_store.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("overlays preview fields without restoring stale asset paths", () {
    final store = LibraryPreviewStore();
    addTearDown(store.dispose);
    final original = _asset("one", sourcePath: "C:\\Pictures\\old.jpg");
    final renamed = _asset("one", sourcePath: "C:\\Pictures\\renamed.jpg");

    store.publish(
      original.withPreview(
        previewPath: "C:\\Cache\\one.jpg",
        width: 320,
        height: 180,
        previewStatus: LibraryPreviewStatus.ready,
      ),
    );

    final resolved = store.resolve(renamed);
    expect(resolved.sourcePath, renamed.sourcePath);
    expect(resolved.displayPath, renamed.displayPath);
    expect(resolved.relativePath, renamed.relativePath);
    expect(resolved.previewPath, "C:\\Cache\\one.jpg");
    expect(resolved.previewStatus, LibraryPreviewStatus.ready);
  });

  test("retains demanded identities and notifies only evicted entries", () {
    final store = LibraryPreviewStore();
    addTearDown(store.dispose);
    final first = _asset("first");
    final second = _asset("second");
    var firstChanges = 0;
    var secondChanges = 0;
    final firstSubscription = store
        .changesFor(first.locationId)
        .listen((_) => firstChanges++);
    final secondSubscription = store
        .changesFor(second.locationId)
        .listen((_) => secondChanges++);
    addTearDown(firstSubscription.cancel);
    addTearDown(secondSubscription.cancel);

    store.publish(_ready(first));
    store.publish(_ready(second));
    store.retain([first.locationId]);

    expect(firstChanges, 1);
    expect(secondChanges, 2);
    expect(store.resolve(first).previewStatus, LibraryPreviewStatus.ready);
    expect(store.resolve(second).previewStatus, LibraryPreviewStatus.pending);
  });

  test("retains failure evidence after an identity leaves demand", () {
    final store = LibraryPreviewStore();
    addTearDown(store.dispose);
    final asset = _asset("failed");

    store.publish(_failed(asset));
    store.retain(const []);

    final resolved = store.resolve(asset);
    expect(resolved.previewStatus, LibraryPreviewStatus.failed);
    expect(resolved.previewIssueCode, "decoder_failed");
  });

  test("bounds retained failure evidence", () {
    final store = LibraryPreviewStore(maxRetainedFailures: 1);
    addTearDown(store.dispose);
    final first = _asset("first-failure");
    final second = _asset("second-failure");

    store.publish(_failed(first));
    store.retain(const []);
    store.publish(_failed(second));
    store.retain(const []);

    expect(store.resolve(first).previewStatus, LibraryPreviewStatus.pending);
    expect(store.resolve(second).previewStatus, LibraryPreviewStatus.failed);
  });
}

LibraryAsset _asset(String id, {String? sourcePath}) {
  final path = sourcePath ?? "C:\\Pictures\\$id.jpg";
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: "location-$id",
    rootId: "root-1",
    sourcePath: path,
    displayPath: path,
    relativePath: path.split("\\").last,
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 90,
    previewStatus: LibraryPreviewStatus.pending,
    fileIdentity: const LibraryFileIdentityEvidence(
      scheme: "windows-file-id-v1",
      value: "volume:file",
    ),
  );
}

LibraryAsset _ready(LibraryAsset asset) {
  return asset.withPreview(
    previewPath: "C:\\Cache\\${asset.locationId}.jpg",
    width: asset.width,
    height: asset.height,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

LibraryAsset _failed(LibraryAsset asset) {
  return asset.withPreview(
    previewPath: "",
    width: asset.width,
    height: asset.height,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: "decoder_failed",
    previewIssueMessage: "Decoder failed",
  );
}
