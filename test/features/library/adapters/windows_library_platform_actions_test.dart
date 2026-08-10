import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "passes the Explorer select switch separately from its target",
    () async {
      final launches = <List<String>>[];
      final actions = WindowsLibraryPlatformActions.withLauncher((
        arguments,
      ) async {
        launches.add(List.of(arguments));
      });

      await actions.revealFile(r"\\?\C:\Pictures\sample image.png");
      await actions.revealDirectory(r"\\?\C:\Pictures");
      await actions.revealLibraryFolder(
        r"\\?\G:\CloudLibrary\图片\",
        r"本机照片\2026\08",
      );
      await actions.revealFile(r"\\?\UNC\server\share\sample.png");

      expect(launches, [
        [r"/select,", r"C:\Pictures\sample image.png"],
        [r"/select,", r"C:\Pictures"],
        [r"/select,", r"G:\CloudLibrary\图片\本机照片\2026\08"],
        [r"/select,", r"\\server\share\sample.png"],
      ]);
    },
  );
}
