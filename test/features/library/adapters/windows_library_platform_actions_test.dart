import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "passes normalized Unicode paths directly to the Shell selection boundary",
    () async {
      final launches = <String>[];
      final actions = WindowsLibraryPlatformActions.withRevealer((path) async {
        launches.add(path);
      });

      await actions.revealFile(r"\\?\C:\Pictures\sample image.png");
      await actions.revealDirectory(r"\\?\C:\Pictures");
      await actions.revealLibraryFolder(
        r"\\?\G:\CloudLibrary\图片\",
        r"本机照片\2026\08",
      );
      await actions.revealFile(r"\\?\UNC\server\share\sample.png");

      expect(launches, [
        r"C:\Pictures\sample image.png",
        r"C:\Pictures",
        r"G:\CloudLibrary\图片\本机照片\2026\08",
        r"\\server\share\sample.png",
      ]);
    },
  );
}
