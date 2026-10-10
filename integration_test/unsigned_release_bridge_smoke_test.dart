import "dart:io";

import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:flutter/services.dart";
import "package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    final path = Platform.environment["CEDARFLAKE_AME_TEST_LIBRARY_PATH"];
    if (path == null || path.isEmpty) {
      throw StateError(
        "An explicitly staged unsigned Release library is required",
      );
    }
    return RustLib.init(
      externalLibrary: ExternalLibrary.open(
        File(path).absolute.path,
        debugInfo: "unsigned Release bridge without catalog initialization",
      ),
    );
  });

  tearDownAll(RustLib.dispose);

  testWidgets("loads the unsigned Release bridge and Windows accent channel", (
    tester,
  ) async {
    const channel = MethodChannel("cedarflake_ame/system_theme");
    final color = await channel.invokeMethod<Object?>("getAccentColor");

    expect(color, isA<int>());
    expect(color as int, inInclusiveRange(0, 0xFFFFFFFF));
  });
}
