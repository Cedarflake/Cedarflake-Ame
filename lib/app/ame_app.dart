import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../features/library/presentation/unified_library_screen.dart";
import "../features/settings/application/ame_preferences.dart";
import "presentation/ame_localizations.dart";
import "presentation/ame_system_theme.dart";
import "presentation/ame_theme.dart";

class AmeApp extends ConsumerWidget {
  const AmeApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final preferences = ref.watch(amePreferencesControllerProvider);
    return AmeSystemThemeBuilder(
      builder: (context, seedColor) => MaterialApp(
        debugShowCheckedModeBanner: false,
        title: "Cedarflake Ame",
        locale: ameLocale,
        supportedLocales: ameSupportedLocales,
        localizationsDelegates: ameLocalizationsDelegates,
        theme: buildAmeTheme(seedColor: seedColor),
        darkTheme: buildAmeTheme(
          brightness: Brightness.dark,
          seedColor: seedColor,
        ),
        themeMode: ameThemeMode(preferences.theme),
        home: const UnifiedLibraryScreen(),
      ),
    );
  }
}
