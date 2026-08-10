import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
