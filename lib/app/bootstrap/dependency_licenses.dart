import "package:flutter/foundation.dart";
import "package:flutter/services.dart";

void registerDependencyLicenses() {
  LicenseRegistry.addLicense(() async* {
    final notices = await rootBundle.loadString(
      "assets/licenses/similarity.txt",
    );
    yield LicenseEntryWithLineBreaks(["Ame similarity engine"], notices);
    final classification = await rootBundle.loadString(
      "assets/licenses/classification_schema.txt",
    );
    yield LicenseEntryWithLineBreaks(["ONNX", "prost", "half"], classification);
    final loading = await rootBundle.loadString(
      "assets/licenses/material_loading_indicator.txt",
    );
    yield LicenseEntryWithLineBreaks(["Material loading indicator"], loading);
  });
}
