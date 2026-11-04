import "dart:io";

import "package:cedarflake_ame/features/storage/adapters/platform/storage_directory_picker.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "catalog and cache selection retain their previous parent directory",
    () async {
      final parents = <String>[];
      final picker = PlatformStorageDirectoryPicker.withSelector((
        directory,
      ) async {
        parents.add(directory);
        return "chosen";
      });
      for (final path in [
        r"C:\图库\catalog\ame.sqlite3",
        r"C:\图库\cache\previews",
        r"\\server\share\previews",
        r"C:\图库\cache\",
      ]) {
        expect(await picker.pick(currentPath: path), "chosen");
        expect(parents.last, File(path).parent.path);
        expect(parents.last, Directory(path).parent.path);
      }
    },
  );

  test(
    "cancellation and platform failure keep their original result",
    () async {
      final cancelled = PlatformStorageDirectoryPicker.withSelector(
        (_) async => null,
      );
      expect(await cancelled.pick(currentPath: r"C:\cache\previews"), isNull);
      final failure = FileSystemException("picker unavailable");
      final failing = PlatformStorageDirectoryPicker.withSelector(
        (_) async => throw failure,
      );
      await expectLater(
        failing.pick(currentPath: r"C:\cache\previews"),
        throwsA(same(failure)),
      );
    },
  );
}
