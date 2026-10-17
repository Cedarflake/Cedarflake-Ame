import "dart:io";

import "package:cedarflake_ame/features/library/adapters/windows_shell_reveal.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "invalid paths are rejected before entering the Shell",
    () async {
      for (final path in ["", "C:\\fixture\\one.png\u0000two.png"]) {
        await expectLater(revealWindowsShellItem(path), throwsArgumentError);
      }
    },
    skip: !Platform.isWindows,
  );

  test(
    "a missing Shell item returns its native failure on repeated calls",
    () async {
      final directory = await Directory.systemTemp.createTemp(
        "ame-shell-reveal-",
      );
      addTearDown(() => directory.delete());
      final missing = "${directory.path}\\不存在, image.png";
      for (var attempt = 0; attempt < 3; attempt++) {
        await expectLater(
          revealWindowsShellItem(missing),
          throwsA(
            isA<PlatformException>()
                .having(
                  (error) => error.code,
                  "code",
                  "windows_shell_reveal_failed",
                )
                .having(
                  (error) => error.details,
                  "HRESULT",
                  contains("hresult"),
                ),
          ),
        );
      }
    },
    skip: !Platform.isWindows,
  );
}
