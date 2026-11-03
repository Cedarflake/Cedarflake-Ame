import "package:flutter/material.dart";

import "../domain/ame_preferences.dart";

ThemeMode ameThemeMode(AmeThemePreference preference) => switch (preference) {
  AmeThemePreference.system => ThemeMode.system,
  AmeThemePreference.light => ThemeMode.light,
  AmeThemePreference.dark => ThemeMode.dark,
};
