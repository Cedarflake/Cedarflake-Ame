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
  return ThemeData(
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
}
