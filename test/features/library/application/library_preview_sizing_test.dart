import "package:cedarflake_ame/features/library/application/library_preview_sizing.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "verified sizes survive leaving demand within a bounded recent cache",
    () {
      final sizing = LibraryPreviewSizing(maxRetainedVerified: 2);
      final first = _asset("first");
      final second = _asset("second");
      final third = _asset("third");
      sizing.recordVerified(first, 256);
      sizing.recordVerified(second, 128);
      sizing.retainDemand({});
      expect(sizing.needsVerification(first, 128), isFalse);
      sizing.recordVerified(third, 128);
      expect(sizing.needsVerification(second, 128), isTrue);
      expect(sizing.needsVerification(first, 256), isFalse);
      expect(sizing.needsVerification(first, 512), isTrue);
      expect(
        sizing.needsVerification(_asset("first", generation: 2), 128),
        isTrue,
      );
      sizing.invalidateRoot(first.rootId);
      expect(sizing.needsVerification(first, 128), isTrue);
      expect(sizing.needsVerification(third, 128), isTrue);
    },
  );

  test(
    "size failures keep their existing demand and explicit retry lifetime",
    () {
      final sizing = LibraryPreviewSizing();
      final asset = _asset("failed");
      sizing.recordFailure(asset, 256);
      expect(sizing.needsVerification(asset, 256), isFalse);
      sizing.retainDemand({});
      expect(sizing.needsVerification(asset, 256), isTrue);
      sizing.recordFailure(asset, 256);
      sizing.allowRetry(asset.locationId);
      expect(sizing.needsVerification(asset, 256), isTrue);
      sizing.recordVerified(asset, 256);
      sizing.clear();
      expect(sizing.needsVerification(asset, 256), isTrue);
    },
  );
}

LibraryAsset _asset(String id, {int generation = 1}) => LibraryAsset(
  assetId: "asset-$id",
  locationId: "location-$id",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unused/$id.jpg",
  displayPath: "$id.jpg",
  relativePath: "$id.jpg",
  previewPath: "cache/$id.jpg",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.from(generation),
  width: 640,
  height: 480,
  previewStatus: LibraryPreviewStatus.ready,
);
