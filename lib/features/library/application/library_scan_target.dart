import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_scanner.dart";

sealed class LibraryScanTarget {
  const LibraryScanTarget();

  LibraryTaskKind? get taskKind;

  Stream<LibraryScanUpdate> open(
    LibraryScanner scanner,
    RecoverableLibraryScan scan,
  );
}

class LibraryDirectoryScan extends LibraryScanTarget {
  const LibraryDirectoryScan();

  @override
  LibraryTaskKind? get taskKind => null;

  @override
  Stream<LibraryScanUpdate> open(
    LibraryScanner scanner,
    RecoverableLibraryScan scan,
  ) => scanner.scan(
    scanId: scan.scanId,
    rootPath: scan.rootPath,
    itemLimit: scan.itemLimit,
    entryLimit: scan.entryLimit,
    previewEdge: scan.previewEdge,
  );
}

class LibraryRootRelocation extends LibraryScanTarget {
  const LibraryRootRelocation(this.root);

  final LibraryRoot root;

  @override
  LibraryTaskKind get taskKind => LibraryTaskKind.update;

  @override
  Stream<LibraryScanUpdate> open(
    LibraryScanner scanner,
    RecoverableLibraryScan scan,
  ) {
    if (scanner case final LibraryRootRelocationScanner relocator) {
      return relocator.relocate(
        root: root,
        scanId: scan.scanId,
        rootPath: scan.rootPath,
        previewEdge: scan.previewEdge,
      );
    }
    throw const LibraryScanFailure(
      code: "library_root_relocation_unavailable",
      message: "当前扫描服务不支持重新定位图库。",
    );
  }
}
