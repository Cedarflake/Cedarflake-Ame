import "package:flutter/widgets.dart";

import "../../../app/presentation/ame_localizations.dart";
import "../domain/ame_preferences.dart";

Locale? amePreferredLocale(AmeLanguagePreference preference) =>
    switch (preference) {
      AmeLanguagePreference.system => null,
      AmeLanguagePreference.zhHans => ameChineseLocale,
      AmeLanguagePreference.en => ameEnglishLocale,
    };
