import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("uses one regular weight for application typography", () {
    for (final brightness in Brightness.values) {
      final theme = buildAmeTheme(brightness: brightness);
      for (final textTheme in [theme.textTheme, theme.primaryTextTheme]) {
        for (final style in _styles(textTheme)) {
          expect(style.fontWeight, FontWeight.w400);
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
