import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../features/library/presentation/library_model_installation_notifications.dart";
import "../features/library/presentation/pages/unified_library_screen.dart";
import "../features/library/presentation/widgets/library_analysis_activity.dart";
import "../features/settings/application/ame_preferences.dart";
import "../features/settings/presentation/ame_theme_preference.dart";
import "presentation/ame_localizations.dart";
import "presentation/ame_scroll_motion_theme.dart";
import "presentation/ame_system_theme.dart";
import "presentation/ame_theme.dart";

class AmeApp extends ConsumerWidget {
  const AmeApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryModelInstallationNotificationsProvider);
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
        home: const LibraryAnalysisActivity(
          child: AmeScrollMotionTheme(child: UnifiedLibraryScreen()),
        ),
      ),
    );
  }
}
