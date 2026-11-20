import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/l10n/generated/app_localizations.dart"
    show lookupAppLocalizations;
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/presentation/library_category_label.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_date.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final english = lookupAppLocalizations(ameEnglishLocale);
  final chinese = lookupAppLocalizations(ameChineseLocale);

  test("resolves supported system languages in preference order", () {
    Locale resolve(List<Locale> locales) =>
        resolveAmeLocales(locales, ameSupportedLocales);
    expect(
      resolve(const [Locale("fr"), Locale("zh", "SG"), Locale("en")]),
      ameChineseLocale,
    );
    expect(
      resolve(const [Locale("en", "GB"), Locale("zh", "CN")]),
      ameEnglishLocale,
    );
    expect(resolve(const [Locale("zh", "TW"), Locale("en")]), ameEnglishLocale);
    expect(
      resolve(const [
        Locale.fromSubtags(
          languageCode: "zh",
          scriptCode: "Hans",
          countryCode: "HK",
        ),
      ]),
      ameChineseLocale,
    );
    expect(resolve(const [Locale("fr")]), ameEnglishLocale);
    expect(resolve(const []), ameEnglishLocale);
  });

  test(
    "formats gallery headers, counts and unknown dates in the selected language",
    () {
      expect(english.photoCount(1), "1 photo");
      expect(english.photoCount(1200), "1,200 photos");
      expect(galleryDateLabel(english, "2026-02-03"), "Feb 3, 2026");
      expect(galleryDateLabel(chinese, "2026-02-03"), "2026年2月3日");
      expect(galleryDateLabel(english, "2026-02-30"), "2026-02-30");
      expect(
        galleryDateLabel(english, "2026-02", monthOnly: true),
        "February 2026",
      );
      expect(galleryDateLabel(chinese, null), chinese.unknownCaptureDate);
    },
  );

  test("only shipped category names follow the display language", () {
    LibraryClassificationCategory category(String id, String label) =>
        LibraryClassificationCategory(
          id: id,
          label: label,
          descriptions: const ["model input"],
        );
    final original = category("illustration", "插画");
    expect(libraryCategoryLabel(english, original), "Illustration");
    expect(libraryCategoryLabel(chinese, original), "插画");
    expect(
      libraryCategoryLabel(english, category("illustration", "我的插画")),
      "我的插画",
    );
    expect(libraryCategoryLabel(english, category("discovered-1", "插画")), "插画");
    expect(original.descriptions, ["model input"]);
  });
}
