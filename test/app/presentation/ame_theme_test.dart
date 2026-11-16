import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/app/presentation/ame_workbench_surfaces.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("workbench tonal hierarchy remains distinct in both appearances", () {
    for (final brightness in Brightness.values) {
      final colors = buildAmeTheme(brightness: brightness).colorScheme;
      final surfaces = AmeWorkbenchSurfaces.from(colors);
      expect(
        surfaces.navigation.computeLuminance(),
        lessThan(surfaces.chrome.computeLuminance()),
      );
      expect(
        surfaces.chrome.computeLuminance(),
        lessThan(surfaces.content.computeLuminance()),
      );
      for (final (text, surface) in [
        (colors.onSurface, surfaces.chrome),
        (colors.onSurface, surfaces.content),
        (colors.onSurfaceVariant, surfaces.chrome),
        (colors.onPrimaryContainer, colors.primaryContainer),
        (colors.onSecondaryContainer, colors.secondaryContainer),
      ]) {
        final foreground = text.computeLuminance();
        final background = surface.computeLuminance();
        final contrast = foreground > background
            ? (foreground + .05) / (background + .05)
            : (background + .05) / (foreground + .05);
        expect(contrast, greaterThanOrEqualTo(4.5));
      }
    }
  });

  test("uses a three-level application typography hierarchy", () {
    for (final brightness in Brightness.values) {
      final theme = buildAmeTheme(brightness: brightness);
      for (final textTheme in [theme.textTheme, theme.primaryTextTheme]) {
        for (final style in _displayAndHeadlineStyles(textTheme)) {
          expect(style.fontWeight, ameFontWeightSemibold);
        }
        expect(textTheme.titleLarge?.fontWeight, ameFontWeightSemibold);
        expect(textTheme.titleMedium?.fontWeight, ameFontWeightMedium);
        expect(textTheme.titleSmall?.fontWeight, ameFontWeightMedium);
        for (final style in _bodyStyles(textTheme)) {
          expect(style.fontWeight, ameFontWeightRegular);
        }
        for (final style in _labelStyles(textTheme)) {
          expect(style.fontWeight, ameFontWeightMedium);
        }
      }
      expect(
        theme.listTileTheme.titleTextStyle?.fontWeight,
        ameFontWeightRegular,
      );
    }
  });

  test("derives both brightness variants from the system accent seed", () {
    const seedColor = Color(0xFFE97132);
    for (final brightness in Brightness.values) {
      final theme = buildAmeTheme(brightness: brightness, seedColor: seedColor);
      final expected = ColorScheme.fromSeed(
        seedColor: seedColor,
        brightness: brightness,
        dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
      );
      expect(theme.colorScheme.primary, expected.primary);
      expect(theme.colorScheme.secondary, expected.secondary);
      expect(theme.colorScheme.brightness, brightness);
    }
  });

  test("uses the Windows Simplified Chinese UI font chain", () {
    for (final brightness in Brightness.values) {
      final theme = buildAmeTheme(brightness: brightness);
      for (final textTheme in [theme.textTheme, theme.primaryTextTheme]) {
        for (final style in _styles(textTheme)) {
          expect(style.fontFamily, ameFontFamily);
          expect(style.fontFamilyFallback, ameFontFamilyFallback);
        }
      }
    }
  });

  test("uses the shared bottom notification surface for snackbars", () {
    final theme = buildAmeTheme();
    final snackBarTheme = theme.snackBarTheme;

    expect(
      snackBarTheme.backgroundColor,
      theme.colorScheme.surfaceContainerHigh,
    );
    expect(snackBarTheme.contentTextStyle?.color, theme.colorScheme.onSurface);
    expect(snackBarTheme.actionTextColor, theme.colorScheme.primary);
    expect(snackBarTheme.elevation, ameNotificationElevation);
    expect(snackBarTheme.behavior, SnackBarBehavior.floating);
    expect(snackBarTheme.width, ameNotificationWidth);
    final shape = snackBarTheme.shape! as RoundedRectangleBorder;
    expect(shape.borderRadius, BorderRadius.circular(ameNotificationRadius));
  });
}

Iterable<TextStyle> _styles(TextTheme textTheme) sync* {
  yield textTheme.displayLarge!;
  yield textTheme.displayMedium!;
  yield textTheme.displaySmall!;
  yield textTheme.headlineLarge!;
  yield textTheme.headlineMedium!;
  yield textTheme.headlineSmall!;
  yield textTheme.titleLarge!;
  yield textTheme.titleMedium!;
  yield textTheme.titleSmall!;
  yield textTheme.bodyLarge!;
  yield textTheme.bodyMedium!;
  yield textTheme.bodySmall!;
  yield textTheme.labelLarge!;
  yield textTheme.labelMedium!;
  yield textTheme.labelSmall!;
}

Iterable<TextStyle> _displayAndHeadlineStyles(TextTheme textTheme) sync* {
  yield textTheme.displayLarge!;
  yield textTheme.displayMedium!;
  yield textTheme.displaySmall!;
  yield textTheme.headlineLarge!;
  yield textTheme.headlineMedium!;
  yield textTheme.headlineSmall!;
}

Iterable<TextStyle> _bodyStyles(TextTheme textTheme) sync* {
  yield textTheme.bodyLarge!;
  yield textTheme.bodyMedium!;
  yield textTheme.bodySmall!;
}

Iterable<TextStyle> _labelStyles(TextTheme textTheme) sync* {
  yield textTheme.labelLarge!;
  yield textTheme.labelMedium!;
  yield textTheme.labelSmall!;
}
