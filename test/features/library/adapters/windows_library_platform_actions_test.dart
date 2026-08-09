import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("reveals files and folders with one Explorer select argument", () async {
    final launches = <List<String>>[];
    final actions = WindowsLibraryPlatformActions.withLauncher((
      arguments,
    ) async {
      launches.add(List.of(arguments));
    });

    await actions.revealFile(r"\\?\C:\Pictures\sample image.png");
    await actions.revealDirectory(r"\\?\C:\Pictures");
    await actions.revealLibraryFolder(
      r"\\?\cloud-primary\",
      r"本机照片\2026\08",
    );
    await actions.revealFile(r"\\?\UNC\server\share\sample.png");

    expect(launches, [
      [r"/select,C:\Pictures\sample image.png"],
      [r"/select,C:\Pictures"],
      [r"/select,cloud-primary\本机照片\2026\08"],
      [r"/select,\\server\share\sample.png"],
    ]);
  });
}
