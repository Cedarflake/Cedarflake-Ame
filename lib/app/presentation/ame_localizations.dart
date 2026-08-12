import "package:flutter/material.dart";
import "package:flutter_localizations/flutter_localizations.dart";

const ameLocale = Locale.fromSubtags(
  languageCode: "zh",
  scriptCode: "Hans",
  countryCode: "CN",
);

const ameSupportedLocales = <Locale>[ameLocale];

const ameLocalizationsDelegates = <LocalizationsDelegate<dynamic>>[
  GlobalMaterialLocalizations.delegate,
  GlobalWidgetsLocalizations.delegate,
  GlobalCupertinoLocalizations.delegate,
];
