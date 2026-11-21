import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../../app/window/ame_window_chrome.dart";
import "library_workbench_metrics.dart";

/// One backing surface joins the title bar and contextual sidebar.
class LibraryWorkbenchFrame extends StatelessWidget {
  const LibraryWorkbenchFrame({
    required this.navigation,
    required this.content,
    required this.sidebarToggle,
    required this.globalBar,
    super.key,
  });

  final Widget navigation;
  final Widget content;
  final Widget sidebarToggle;
  final Widget globalBar;

  @override
  Widget build(BuildContext context) {
    final surfaces = AmeWorkbenchSurfaces.of(context);
    return ColoredBox(
      color: surfaces.navigation,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          IntrinsicWidth(
            child: Column(
              children: [
                SizedBox(
                  height: LibraryWorkbenchMetrics.globalBarHeight,
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      const Positioned.fill(
                        child: AmeWindowDragRegion(child: SizedBox.expand()),
                      ),
                      Center(child: sidebarToggle),
                    ],
                  ),
                ),
                Expanded(child: navigation),
              ],
            ),
          ),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: surfaces.chrome,
                borderRadius: const BorderRadius.horizontal(
                  left: Radius.circular(24),
                ),
              ),
              child: Column(
                children: [
                  globalBar,
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.only(right: 8, bottom: 8),
                      child: content,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
