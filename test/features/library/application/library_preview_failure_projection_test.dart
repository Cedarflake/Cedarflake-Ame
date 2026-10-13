import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final code in [
    "preview_source_open_failed",
    "preview_write_failed",
    "preview_cache_budget_exceeded",
  ]) {
    test("$code does not replace a retained ready preview", () async {
      final publications = <LibraryAsset>[];
      final queue = LibraryPreviewQueue(
        previewer: _FailingPreviewer(code),
        previewEdge: 512,
        maxActive: 1,
        onResult: publications.add,
      );
      addTearDown(queue.dispose);
      final outcome = await queue.retry(_asset(LibraryPreviewStatus.ready));
      expect(outcome, LibraryPreviewRequestOutcome.failed);
      expect(publications, isEmpty);
    });
    test(
      "$code during automatic size verification keeps current pixels",
      () async {
        final publications = <LibraryAsset>[];
        final queue = LibraryPreviewQueue(
          previewer: _FailingPreviewer(code),
          previewEdge: 512,
          maxActive: 1,
          onResult: publications.add,
        );
        addTearDown(queue.dispose);
        queue.request(_asset(LibraryPreviewStatus.ready), ensureSize: true);
        await Future<void>.delayed(Duration.zero);
        expect(publications, isEmpty);
      },
    );
  }

  test("a rejected ready request cannot report a current failure", () async {
    final publications = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: const _FailingPreviewer("preview_source_open_failed"),
      previewEdge: 512,
      maxActive: 1,
      canPublishResult: (_) => false,
      onResult: publications.add,
    );
    addTearDown(queue.dispose);
    expect(
      await queue.retry(_asset(LibraryPreviewStatus.ready)),
      LibraryPreviewRequestOutcome.superseded,
    );
    expect(publications, isEmpty);
  });

  test("confirmed corruption can revoke an existing ready preview", () async {
    final failed = _asset(LibraryPreviewStatus.failed).withPreview(
      previewPath: "",
      width: 160,
      height: 120,
      previewStatus: LibraryPreviewStatus.failed,
      previewIssueCode: "image_decode_failed",
      previewIssueMessage: "Confirmed corrupt source",
    );
    final publications = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: _ResolvedPreviewer(failed),
      previewEdge: 512,
      maxActive: 1,
      onResult: publications.add,
    );
    addTearDown(queue.dispose);
    expect(
      await queue.retry(_asset(LibraryPreviewStatus.ready)),
      LibraryPreviewRequestOutcome.failed,
    );
    expect(publications.single.previewStatus, LibraryPreviewStatus.failed);
    expect(publications.single.previewPath, isEmpty);
    expect(publications.single.previewIssueCode, "image_decode_failed");
  });

  test("a missing active catalog location retires the request", () async {
    final publications = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: const _FailingPreviewer("preview_location_not_found"),
      previewEdge: 512,
      maxActive: 1,
      onResult: publications.add,
    );
    addTearDown(queue.dispose);
    expect(
      await queue.retry(_asset(LibraryPreviewStatus.pending)),
      LibraryPreviewRequestOutcome.superseded,
    );
    expect(publications, isEmpty);
  });

  test("a source lock without a usable preview remains actionable", () async {
    final publications = <LibraryAsset>[];
    final queue = LibraryPreviewQueue(
      previewer: const _FailingPreviewer("preview_source_open_failed"),
      previewEdge: 512,
      maxActive: 1,
      onResult: publications.add,
    );
    addTearDown(queue.dispose);
    expect(
      await queue.retry(_asset(LibraryPreviewStatus.pending)),
      LibraryPreviewRequestOutcome.failed,
    );
    expect(publications.single.previewStatus, LibraryPreviewStatus.failed);
    expect(publications.single.previewIssueCode, "preview_source_open_failed");
  });
}

LibraryAsset _asset(LibraryPreviewStatus status) => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  sourcePath: "C:\\Pictures\\source.png",
  displayPath: "C:\\Pictures\\source.png",
  relativePath: "source.png",
  previewPath: status == LibraryPreviewStatus.ready
      ? "C:\\Cache\\preview.png"
      : "",
  fileSize: BigInt.from(64),
  modifiedUnixMs: 1,
  width: 160,
  height: 120,
  previewStatus: status,
);

class _FailingPreviewer implements LibraryPreviewer {
  const _FailingPreviewer(this.code);

  final String code;

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
  }) async {
    throw LibraryPreviewFailure(code: code, message: "Controlled $code");
  }
}

class _ResolvedPreviewer implements LibraryPreviewer {
  _ResolvedPreviewer(this.result);

  final LibraryAsset result;

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
  }) async => result;
}
