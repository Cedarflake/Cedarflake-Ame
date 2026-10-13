import "dart:async";

import "package:cedarflake_ame/features/library/application/library_scan_source_admission.dart";
import "package:cedarflake_ame/features/library/application/library_scan_target.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  test("a combined destination cannot inherit an old task's identity", () {
    final admission = _admission();
    expect(admission.needsRefresh, isTrue);
    expect(admission.initialTaskKind, LibraryTaskKind.import);
    expect(
      admission.hasAdmittedExistingRoot([
        otherPublishedRoot,
        _root("combined", r"C:\Moved", published: true),
      ], displayRootPath: r"C:\Moved"),
      isFalse,
    );
  });

  test(
    "only a previously published root can turn an import into an update",
    () {
      final roots = [retainedScanRoot, otherPublishedRoot];
      final admission = _admission(roots: roots);
      roots[0] = _root("retained", r"C:\Moved", published: true);
      expect(
        admission.hasAdmittedExistingRoot(roots, displayRootPath: r"C:\Moved"),
        isFalse,
      );
      roots[1] = _root("other", r"C:\Moved", published: true);
      expect(
        admission.hasAdmittedExistingRoot(roots, displayRootPath: r"C:\Moved"),
        isTrue,
      );
    },
  );

  test("a known unpublished path takes precedence over identity discovery", () {
    final admission = _admission(path: retainedScanRoot.path);
    expect(admission.needsRefresh, isFalse);
    expect(admission.initialTaskKind, LibraryTaskKind.import);
  });

  test("an ordinary registered update needs no admission refresh", () {
    final admission = _admission(path: otherPublishedRoot.path);
    expect(admission.needsRefresh, isFalse);
    expect(admission.initialTaskKind, LibraryTaskKind.update);
  });

  test("resume never rediscovers another source binding", () {
    expect(_admission(isResuming: true).needsRefresh, isFalse);
  });

  test("an empty library has no prior source binding to recover", () {
    expect(_admission(roots: []).needsRefresh, isFalse);
  });

  test("explicit replacement retains update intent before admission", () {
    final admission = _admission(
      target: const LibraryRootRelocation(otherPublishedRoot),
    );
    expect(admission.needsRefresh, isTrue);
    expect(admission.initialTaskKind, LibraryTaskKind.update);
    expect(
      admission.hasAdmittedExistingRoot([
        otherPublishedRoot,
      ], displayRootPath: r"C:\Moved"),
      isFalse,
    );
  });

  test("start and terminal cleanup share one admitted-source read", () async {
    final admission = _admission();
    final read = Completer<bool>();
    var reads = 0;
    Future<bool> reload() {
      reads += 1;
      return read.future;
    }

    final started = admission.refresh(reload);
    final terminal = admission.refresh(reload);
    expect(reads, 1);
    read.complete(false);
    expect(await started, isFalse);
    expect(await terminal, isFalse);
    expect(await admission.refresh(reload), isFalse);
    expect(reads, 1);
  });
}

LibraryScanSourceAdmission _admission({
  String path = r"C:\Moved",
  List<LibraryRoot> roots = const [retainedScanRoot, otherPublishedRoot],
  LibraryScanTarget target = const LibraryDirectoryScan(),
  bool isResuming = false,
}) => LibraryScanSourceAdmission(
  scan: RecoverableLibraryScan(
    scanId: "new-scan",
    rootPath: path,
    displayRootPath: path,
    previewEdge: 384,
    visitedEntries: 0,
    acceptedItems: 0,
    issueCount: 0,
  ),
  roots: roots,
  target: target,
  isResuming: isResuming,
);

LibraryRoot _root(String id, String path, {required bool published}) =>
    LibraryRoot(
      id: id,
      path: path,
      displayPath: path,
      activeScanId: published ? "published" : null,
      createdUnixMs: 1,
      assetCount: 6,
      issueCount: 0,
      availability: LibraryRootAvailability.available,
    );
