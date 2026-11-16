import "package:cedarflake_ame/app/window/ame_build_identity.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("retains the executable prerelease and build number", () {
    const identity = AmeBuildIdentity(name: "0.1.0-beta.1", number: "4");
    expect(identity.applicationLabel, "Cedarflake Ame 0.1.0-beta.1+4");
    expect(
      identity.startupDiagnostics(const FormatException("catalog fixture")),
      "Cedarflake Ame 0.1.0-beta.1+4\nFormatException: catalog fixture",
    );
  });

  test("an unversioned build does not invent a release identity", () {
    const identity = AmeBuildIdentity(name: null, number: null);
    expect(identity.version, isNull);
    expect(
      identity.startupDiagnostics("loader error"),
      "Cedarflake Ame\nloader error",
    );
  });
}
