import "package:flutter/material.dart";

import "ame_menu.dart";

const ameNotificationWidth = 680.0;
const ameNotificationElevation = 3.0;
const ameNotificationRadius = 16.0;

ThemeData buildAmeTheme({Brightness brightness = Brightness.light}) {
  final colorScheme = ColorScheme.fromSeed(
    seedColor: const Color(0xFF0B57D0),
    brightness: brightness,
    dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
  );
  final baseTheme = ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: colorScheme.surface,
    useMaterial3: true,
    visualDensity: VisualDensity.standard,
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
  return baseTheme.copyWith(
    textTheme: _regularTextTheme(baseTheme.textTheme),
    primaryTextTheme: _regularTextTheme(baseTheme.primaryTextTheme),
  );
}

TextTheme _regularTextTheme(TextTheme textTheme) {
  const fontWeight = FontWeight.w400;
  return textTheme.copyWith(
    displayLarge: textTheme.displayLarge?.copyWith(fontWeight: fontWeight),
    displayMedium: textTheme.displayMedium?.copyWith(fontWeight: fontWeight),
    displaySmall: textTheme.displaySmall?.copyWith(fontWeight: fontWeight),
    headlineLarge: textTheme.headlineLarge?.copyWith(fontWeight: fontWeight),
    headlineMedium: textTheme.headlineMedium?.copyWith(fontWeight: fontWeight),
    headlineSmall: textTheme.headlineSmall?.copyWith(fontWeight: fontWeight),
    titleLarge: textTheme.titleLarge?.copyWith(fontWeight: fontWeight),
    titleMedium: textTheme.titleMedium?.copyWith(fontWeight: fontWeight),
    titleSmall: textTheme.titleSmall?.copyWith(fontWeight: fontWeight),
    bodyLarge: textTheme.bodyLarge?.copyWith(fontWeight: fontWeight),
    bodyMedium: textTheme.bodyMedium?.copyWith(fontWeight: fontWeight),
    bodySmall: textTheme.bodySmall?.copyWith(fontWeight: fontWeight),
    labelLarge: textTheme.labelLarge?.copyWith(fontWeight: fontWeight),
    labelMedium: textTheme.labelMedium?.copyWith(fontWeight: fontWeight),
    labelSmall: textTheme.labelSmall?.copyWith(fontWeight: fontWeight),
  );
}
