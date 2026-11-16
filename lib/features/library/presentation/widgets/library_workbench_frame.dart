import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "library_workbench_metrics.dart";

/// One backing surface joins the title bar and contextual sidebar.
class LibraryWorkbenchFrame extends StatelessWidget {
  const LibraryWorkbenchFrame({
    required this.navigation,
    required this.content,
    this.globalBar,
    super.key,
  });

  final Widget navigation;
  final Widget content;
  final Widget? globalBar;

  @override
  Widget build(BuildContext context) {
    final surfaces = AmeWorkbenchSurfaces.of(context);
    return ColoredBox(
      color: surfaces.navigation,
      child: Stack(
        fit: StackFit.expand,
        children: [
          Positioned(
            left: LibraryWorkbenchMetrics.railWidth,
            top: 0,
            right: 0,
            bottom: 0,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: surfaces.chrome,
                borderRadius: const BorderRadius.horizontal(
                  left: Radius.circular(24),
                ),
              ),
            ),
          ),
          Column(
            children: [
              if (globalBar != null) globalBar!,
              Expanded(
                child: Row(
                  children: [
                    navigation,
                    Expanded(
                      child: Padding(
                        padding: const EdgeInsets.only(right: 8, bottom: 8),
                        child: content,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
