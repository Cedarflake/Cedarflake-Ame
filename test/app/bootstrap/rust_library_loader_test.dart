import "package:cedarflake_ame/app/bootstrap/rust_library_loader.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("resolves the Rust library next to the Windows executable", () {
    expect(
      packagedRustLibraryPath(
        r"C:\Program Files\Cedarflake Ame\cedarflake_ame.exe",
      ),
      r"C:\Program Files\Cedarflake Ame\rust_lib_cedarflake_ame.dll",
    );
  });
}
