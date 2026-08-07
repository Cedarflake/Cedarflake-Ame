import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust_domain;
import "package:flutter_test/flutter_test.dart";

void main() {
  test("maps versioned capture-time evidence without losing provenance", () {
    final asset = mapRustLibraryAsset(
      rust_domain.AssetLocationView(
        assetId: "asset-1",
        locationId: "location-1",
        rootId: "root-1",
        absolutePath: "C:\\Pictures\\capture.jpg",
        relativePath: "capture.jpg",
        previewPath: "",
        fileSize: BigInt.from(123),
        modifiedUnixMs: 456,
        width: 1920,
        height: 1080,
        previewStatus: rust_domain.PreviewStatus.pending,
        metadataEngineId: "kamadak-exif",
        metadataEngineVersion: "0.6.1",
        fileIdentity: const rust_domain.FileIdentityEvidence(
          scheme: "windows-file-id-128-v1",
          value: "0000000000000001:00000000000000000000000000000002",
        ),
        captureTime: const rust_domain.CaptureTimeEvidence(
          localTime: "2025-07-08T09:10:11.123000000",
          offsetMinutes: 480,
          source: rust_domain.CaptureTimeSource.original,
          rawValue: "2025:07:08 09:10:11|123|+08:00",
        ),
      ),
    );

    expect(asset.metadataEngineId, "kamadak-exif");
    expect(asset.metadataEngineVersion, "0.6.1");
    expect(asset.fileIdentity?.scheme, "windows-file-id-128-v1");
    expect(
      asset.fileIdentity?.value,
      "0000000000000001:00000000000000000000000000000002",
    );
    expect(asset.captureTime?.localTime, "2025-07-08T09:10:11.123000000");
    expect(asset.captureTime?.offsetMinutes, 480);
    expect(
      asset.captureTime?.source,
      LibraryCaptureTimeSource.exifDateTimeOriginal,
    );
    expect(asset.captureTime?.rawValue, "2025:07:08 09:10:11|123|+08:00");
  });

  test("preserves an explicitly unknown capture time", () {
    final asset = mapRustLibraryAsset(
      rust_domain.AssetLocationView(
        assetId: "asset-2",
        locationId: "location-2",
        rootId: "root-1",
        absolutePath: "C:\\Pictures\\plain.png",
        relativePath: "plain.png",
        previewPath: "",
        fileSize: BigInt.from(42),
        modifiedUnixMs: 789,
        width: 8,
        height: 6,
        previewStatus: rust_domain.PreviewStatus.pending,
        metadataEngineId: "kamadak-exif",
        metadataEngineVersion: "0.6.1",
      ),
    );

    expect(asset.captureTime, isNull);
  });
}
