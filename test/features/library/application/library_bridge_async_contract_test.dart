import "package:cedarflake_ame/src/rust/api/catalog.dart" as bridge;
import "package:cedarflake_ame/src/rust/domain.dart" as native;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "catalog reads and retained cancellation cannot become synchronous FFI",
    () {
      final Future<native.RecoverableScan?> Function() recoverable =
          bridge.loadRecoverableLibraryScan;
      final Future<native.RecoverableScan?> Function() paused =
          bridge.loadPausedLibraryScan;
      final Future<native.LibraryFolderPage> Function({
        required String rootId,
        required String parentRelativePath,
        required int maxItems,
        native.LibraryFolderCursor? after,
      })
      folders = bridge.loadLibraryFolderPage;
      final Future<void> Function({required String scanId}) cancel =
          bridge.cancelRetainedLibraryScan;
      expect([recoverable, paused, folders, cancel], hasLength(4));
    },
  );
}
