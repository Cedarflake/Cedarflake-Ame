import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/l10n/generated/app_localizations.dart"
    show lookupAppLocalizations;
import "package:material_ui/material_ui.dart";

export "package:cedarflake_ame/app/presentation/ame_message_text.dart";

final testChinese = lookupAppLocalizations(ameChineseLocale);

class LocalizedTestApp extends MaterialApp {
  const LocalizedTestApp({
    super.key,
    super.home,
    super.theme,
    super.locale = ameChineseLocale,
    super.builder,
    super.navigatorKey,
    super.navigatorObservers,
    super.routes,
  }) : super(
         supportedLocales: ameSupportedLocales,
         localizationsDelegates: ameLocalizationsDelegates,
       );
}
