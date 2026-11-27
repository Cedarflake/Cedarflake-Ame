import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/l10n/generated/app_localizations.dart"
    show lookupAppLocalizations;
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/presentation/library_category_label.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_date.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final english = lookupAppLocalizations(ameEnglishLocale);
  final chinese = lookupAppLocalizations(ameChineseLocale);
  final japanese = lookupAppLocalizations(ameJapaneseLocale);

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
    expect(ameChineseLocale.toLanguageTag(), "zh-Hans");
    expect(chinese.localeName, "zh_Hans");
    expect(
      resolve(const [Locale("ja", "JP"), Locale("en")]),
      ameJapaneseLocale,
    );
    expect(
      resolve(const [
        Locale.fromSubtags(
          languageCode: "zh",
          scriptCode: "Hant",
          countryCode: "CN",
        ),
        Locale("ja"),
      ]),
      ameJapaneseLocale,
    );
    expect(
      resolve(const [Locale("zh", "TW"), Locale("ja")]),
      ameJapaneseLocale,
    );
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
      expect(japanese.photoCount(1200), "1,200 枚の写真");
      expect(galleryDateLabel(japanese, "2026-02-03"), "2026年2月3日");
      expect(galleryDateLabel(japanese, "2026-02", monthOnly: true), "2026年2月");
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

  test("automatic category names follow the active language by stable ID", () {
    final categories = <(String, String Function(AppLocalizations))>[
      ("illustration", (strings) => strings.categoryIllustration),
      ("comic", (strings) => strings.categoryComic),
      ("meme", (strings) => strings.categoryMeme),
      ("screenshot", (strings) => strings.categoryScreenshot),
      ("photo", (strings) => strings.categoryPhoto),
      ("other", (strings) => strings.categoryOther),
      (
        "b3642c574eec3080310e55f0847cf59db9a5f76a5f98b6be2aa8749e4573a15e",
        (strings) => strings.categoryFoodPhoto,
      ),
      (
        "aeb8eeab8b8a3926a244a892ea22a910f2869c6cf773619d7b1351424c6510ab",
        (strings) => strings.categoryCoastPhoto,
      ),
      (
        "a284ea94060ab180bca6779062194ffd6c99ca7e9443e53c1019c042455f8808",
        (strings) => strings.categoryCoastIllustration,
      ),
      (
        "951e57580a2a7562cdf95539d06ebb93f09ba39bcffd91950a037a1da1528641",
        (strings) => strings.categoryFlowersPhoto,
      ),
    ];
    for (final (id, localized) in categories) {
      final category = LibraryClassificationCategory(
        id: id,
        label: "Catalog label",
        descriptions: const ["model input"],
      );
      for (final strings in [chinese, english, japanese]) {
        expect(libraryCategoryLabel(strings, category), localized(strings));
      }
      expect(category.id, id);
      expect(category.label, "Catalog label");
      expect(category.descriptions, ["model input"]);
    }
  });

  test("unknown category IDs retain the returned label in every language", () {
    const category = LibraryClassificationCategory(
      id: "unknown-category",
      label: "花草",
      descriptions: ["model input"],
    );
    for (final strings in [chinese, english, japanese]) {
      expect(libraryCategoryLabel(strings, category), category.label);
    }
  });

  test("a scan admission error is displayed in the active language", () {
    const diagnostic = "library_update_in_progress: scan lease held";
    expect(
      LibraryStrings.scanFailureDetail(english, diagnostic),
      "Another folder is updating. Wait for it to finish, then continue.",
    );
    expect(
      LibraryStrings.scanFailureDetail(chinese, diagnostic),
      "另一个目录正在更新，结束后再继续。",
    );
    expect(
      LibraryStrings.scanFailureDetail(japanese, diagnostic),
      "別のフォルダーを更新中です。完了してから続けてください。",
    );
  });
}
