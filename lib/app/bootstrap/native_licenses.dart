import "package:flutter/foundation.dart";
import "package:flutter/services.dart";

void registerNativeLicenses() {
  LicenseRegistry.addLicense(() async* {
    final notices = await rootBundle.loadString(
      "assets/licenses/similarity.txt",
    );
    yield LicenseEntryWithLineBreaks(["Ame similarity engine"], notices);
    final classification = await rootBundle.loadString(
      "assets/licenses/classification_schema.txt",
    );
    yield LicenseEntryWithLineBreaks(["ONNX", "prost", "half"], classification);
  });
}
