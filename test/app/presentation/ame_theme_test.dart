import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/app/presentation/ame_workbench_surfaces.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("workbench text remains readable in both appearances", () {
    for (final seed in [
      const Color(0xFF006A6A),
      const Color(0xFF6750A4),
      const Color(0xFFE97132),
      const Color(0xFF008000),
      const Color(0xFF757575),
    ]) {
      for (final brightness in Brightness.values) {
        final colors = buildAmeTheme(
          brightness: brightness,
          seedColor: seed,
        ).colorScheme;
        final surfaces = AmeWorkbenchSurfaces.from(colors);
        for (final (text, surface) in [
          (colors.onSurface, surfaces.navigation),
          (colors.onSurface, surfaces.chrome),
          (colors.onSurface, surfaces.content),
          (colors.onSurfaceVariant, surfaces.navigation),
          (colors.onSurfaceVariant, surfaces.chrome),
          (colors.onSurfaceVariant, surfaces.content),
          (colors.onPrimaryContainer, colors.primaryContainer),
          (colors.onSecondaryContainer, colors.secondaryContainer),
          for (final opacity in [0.0, 0.08, 0.12])
            (
              surfaces.onSelection,
              Color.alphaBlend(
                surfaces.onSelection.withValues(alpha: opacity),
                surfaces.selection,
              ),
            ),
        ]) {
          final foreground = text.computeLuminance();
          final background = surface.computeLuminance();
          final contrast = foreground > background
              ? (foreground + .05) / (background + .05)
              : (background + .05) / (foreground + .05);
          expect(
            contrast,
            greaterThanOrEqualTo(4.5),
            reason: "$brightness, seed $seed, foreground $text on $surface",
          );
        }
      }
    }
  });
}
