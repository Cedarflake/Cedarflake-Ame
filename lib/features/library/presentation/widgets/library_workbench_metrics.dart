import "package:material_ui/material_ui.dart";

abstract final class LibraryWorkbenchMetrics {
  static const railWidth = 72.0;
  static const contentInset = 8.0;
  static const sidebarRowHeight = 40.0;
  static const sidebarStatusRowHeight = 56.0;
  static const sidebarRowShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(8)),
  );
  static const galleryHeadingInset = 28.0;
  static const iconSize = 28.0;
  static const railIconInset = (railWidth - iconSize) / 2;
}
