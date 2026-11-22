import "package:flutter/widgets.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../features/settings/application/ame_preferences.dart";
import "../../features/settings/presentation/ame_language_preference.dart";
import "../presentation/ame_localizations.dart";
import "../presentation/l10n/generated/app_localizations.dart"
    show lookupAppLocalizations;

AppLocalizations platformDialogLanguage(Ref ref) {
  final preferred = amePreferredLocale(
    ref.read(amePreferencesControllerProvider).language,
  );
  final locale =
      preferred ??
      resolveAmeLocales(
        WidgetsBinding.instance.platformDispatcher.locales,
        ameSupportedLocales,
      );
  return lookupAppLocalizations(locale);
}
