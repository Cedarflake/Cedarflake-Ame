import "package:material_ui/material_ui.dart";

/// Tonal roles keep navigation, contextual chrome and content distinguishable.
@immutable
class AmeWorkbenchSurfaces {
  const AmeWorkbenchSurfaces._({
    required this.navigation,
    required this.chrome,
    required this.content,
    required this.viewerCanvas,
  });

  factory AmeWorkbenchSurfaces.from(ColorScheme colors) {
    final dark = colors.brightness == Brightness.dark;
    return AmeWorkbenchSurfaces._(
      navigation: colors.surfaceContainer,
      chrome: colors.surfaceContainerLow,
      content: colors.surface,
      viewerCanvas: dark ? colors.surfaceContainerLowest : colors.surface,
    );
  }

  factory AmeWorkbenchSurfaces.of(BuildContext context) =>
      AmeWorkbenchSurfaces.from(Theme.of(context).colorScheme);

  final Color navigation;
  final Color chrome;
  final Color content;
  final Color viewerCanvas;
}
