import "package:material_ui/material_ui.dart";

import "l10n/generated/app_localizations.dart";

export "l10n/generated/app_localizations.dart" show AppLocalizations;

const ameChineseLocale = Locale.fromSubtags(
  languageCode: "zh",
  scriptCode: "Hans",
  countryCode: "CN",
);
const ameEnglishLocale = Locale("en");

const ameSupportedLocales = <Locale>[ameEnglishLocale, ameChineseLocale];

const ameLocalizationsDelegates = <LocalizationsDelegate<dynamic>>[
  AppLocalizations.delegate,
  ...GlobalMaterialLocalizations.delegates,
];

Locale resolveAmeLocales(List<Locale>? preferred, Iterable<Locale> supported) {
  for (final locale in preferred ?? const <Locale>[]) {
    if (locale.languageCode == "en") return ameEnglishLocale;
    if (locale.languageCode != "zh") continue;
    if (locale.scriptCode == "Hans") return ameChineseLocale;
    if (locale.scriptCode == null &&
        !const {"TW", "HK", "MO"}.contains(locale.countryCode)) {
      return ameChineseLocale;
    }
  }
  return ameEnglishLocale;
}

extension AmeLocalizedContext on BuildContext {
  AppLocalizations get l10n => AppLocalizations.of(this);
}
