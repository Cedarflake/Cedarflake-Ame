import "package:material_ui/material_ui.dart";

abstract final class LibraryWorkbenchMetrics {
  static const railWidth = 72.0;
  static const contentInset = 8.0;
  static const sidebarRowHeight = 48.0;
  static const sidebarRowShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(8)),
  );
  static const galleryHeadingInset = 28.0;
  static const iconSize = 24.0;
  static const railIconInset = (railWidth - iconSize) / 2;
}
