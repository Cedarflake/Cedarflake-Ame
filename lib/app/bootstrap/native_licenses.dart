import "package:flutter/foundation.dart";
import "package:flutter/services.dart";

void registerNativeLicenses() {
  LicenseRegistry.addLicense(() async* {
    final notices = await rootBundle.loadString(
      "assets/licenses/similarity.txt",
    );
    yield LicenseEntryWithLineBreaks(["Ame similarity engine"], notices);
  });
}
