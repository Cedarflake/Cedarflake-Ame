import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../features/library/presentation/unified_library_screen.dart";
import "../features/settings/application/ame_preferences.dart";
import "ame_theme.dart";

class AmeApp extends ConsumerWidget {
  const AmeApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final preferences = ref.watch(amePreferencesControllerProvider);
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: "Cedarflake Ame",
      theme: buildAmeTheme(),
      darkTheme: buildAmeTheme(brightness: Brightness.dark),
      themeMode: switch (preferences.theme) {
        AmeThemePreference.system => ThemeMode.system,
        AmeThemePreference.light => ThemeMode.light,
        AmeThemePreference.dark => ThemeMode.dark,
      },
      home: const UnifiedLibraryScreen(),
    );
  }
}
