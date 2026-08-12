import "package:flutter/material.dart";

import "../../features/settings/application/ame_preferences.dart";
import "ame_menu.dart";
import "ame_system_theme.dart";
import "ame_typography.dart";

export "ame_typography.dart";

const ameNotificationWidth = 680.0;
const ameNotificationElevation = 3.0;
const ameNotificationRadius = 16.0;

ThemeMode ameThemeMode(AmeThemePreference preference) => switch (preference) {
  AmeThemePreference.system => ThemeMode.system,
  AmeThemePreference.light => ThemeMode.light,
  AmeThemePreference.dark => ThemeMode.dark,
};

ThemeData buildAmeTheme({
  Brightness brightness = Brightness.light,
  Color seedColor = ameFallbackSeedColor,
}) {
  final colorScheme = ColorScheme.fromSeed(
    seedColor: seedColor,
    brightness: brightness,
    dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
  );
  final baseTheme = ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: colorScheme.surface,
    useMaterial3: true,
    visualDensity: VisualDensity.standard,
    fontFamily: ameFontFamily,
    fontFamilyFallback: ameFontFamilyFallback,
    menuTheme: buildAmeMenuTheme(colorScheme),
    menuButtonTheme: buildAmeMenuButtonTheme(),
    popupMenuTheme: buildAmePopupMenuTheme(colorScheme),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: colorScheme.surfaceContainerHigh,
      contentTextStyle: TextStyle(color: colorScheme.onSurface),
      actionTextColor: colorScheme.primary,
      closeIconColor: colorScheme.onSurfaceVariant,
      elevation: ameNotificationElevation,
      behavior: SnackBarBehavior.floating,
      width: ameNotificationWidth,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(ameNotificationRadius),
      ),
    ),
    searchBarTheme: SearchBarThemeData(
      elevation: const WidgetStatePropertyAll(0),
      backgroundColor: WidgetStatePropertyAll(
        colorScheme.surfaceContainerHighest,
      ),
      shape: WidgetStatePropertyAll(
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
      ),
    ),
    tooltipTheme: const TooltipThemeData(
      waitDuration: Duration(milliseconds: 350),
    ),
  );
  final textTheme = _hierarchicalTextTheme(baseTheme.textTheme);
  final primaryTextTheme = _hierarchicalTextTheme(baseTheme.primaryTextTheme);
  return baseTheme.copyWith(
    textTheme: textTheme,
    primaryTextTheme: primaryTextTheme,
    listTileTheme: baseTheme.listTileTheme.copyWith(
      titleTextStyle: textTheme.bodyLarge?.copyWith(
        fontWeight: ameFontWeightMedium,
      ),
    ),
  );
}

TextTheme _hierarchicalTextTheme(TextTheme textTheme) {
  return textTheme.copyWith(
    displayLarge: textTheme.displayLarge?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    displayMedium: textTheme.displayMedium?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    displaySmall: textTheme.displaySmall?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    headlineLarge: textTheme.headlineLarge?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    headlineMedium: textTheme.headlineMedium?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    headlineSmall: textTheme.headlineSmall?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    titleLarge: textTheme.titleLarge?.copyWith(
      fontWeight: ameFontWeightSemibold,
    ),
    titleMedium: textTheme.titleMedium?.copyWith(
      fontWeight: ameFontWeightMedium,
    ),
    titleSmall: textTheme.titleSmall?.copyWith(fontWeight: ameFontWeightMedium),
    bodyLarge: textTheme.bodyLarge?.copyWith(fontWeight: ameFontWeightRegular),
    bodyMedium: textTheme.bodyMedium?.copyWith(
      fontWeight: ameFontWeightRegular,
    ),
    bodySmall: textTheme.bodySmall?.copyWith(fontWeight: ameFontWeightRegular),
    labelLarge: textTheme.labelLarge?.copyWith(fontWeight: ameFontWeightMedium),
    labelMedium: textTheme.labelMedium?.copyWith(
      fontWeight: ameFontWeightMedium,
    ),
    labelSmall: textTheme.labelSmall?.copyWith(fontWeight: ameFontWeightMedium),
  );
}
