import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../../settings/domain/ame_preferences.dart";
import "../../application/library_workbench_controller.dart";
import "../coordination/library_duplicate_workspace.dart";
import "../library_duplicate_strings.dart";
import "library_main_surface.dart";
import "library_sidebar_transition.dart";
import "library_workbench_metrics.dart";

class LibraryDuplicateWorkspaceLayout extends ConsumerWidget {
  const LibraryDuplicateWorkspaceLayout({
    required this.view,
    required this.onViewChanged,
    required this.child,
    super.key,
  });

  final LibraryDuplicateView view;
  final ValueChanged<LibraryDuplicateView> onViewChanged;
  final Widget child;

  static const _destinations = [
    (
      LibraryDuplicateView.exact,
      LibraryDuplicateStrings.pendingGroups,
      Symbols.file_copy_rounded,
    ),
    (
      LibraryDuplicateView.similar,
      LibraryDuplicateStrings.similarGroups,
      Symbols.layers_rounded,
    ),
    (
      LibraryDuplicateView.ignored,
      LibraryDuplicateStrings.ignoredGroups,
      Symbols.visibility_off_rounded,
    ),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) => LayoutBuilder(
    builder: (context, constraints) {
      final surfaces = AmeWorkbenchSurfaces.of(context);
      final content = Expanded(
        child: Padding(
          padding: const EdgeInsets.only(
            left: LibraryWorkbenchMetrics.contentInset,
          ),
          child: LibraryMainSurface(child: child),
        ),
      );
      if (constraints.maxWidth < 840) {
        return Column(
          children: [
            NavigationBar(
              backgroundColor: surfaces.chrome,
              indicatorColor: surfaces.selection,
              selectedIndex: _destinations.indexWhere(
                (item) => item.$1 == view,
              ),
              onDestinationSelected: (index) =>
                  onViewChanged(_destinations[index].$1),
              destinations: [
                for (final (value, label, icon) in _destinations)
                  NavigationDestination(
                    key: ValueKey("duplicate-view-${value.name}"),
                    icon: Icon(
                      icon,
                      color: value == view ? surfaces.onSelection : null,
                    ),
                    label: label,
                  ),
              ],
            ),
            content,
          ],
        );
      }
      final preferences = ref.watch(amePreferencesControllerProvider);
      return LibrarySidebarTransition(
        workspace: LibraryWorkspace.duplicates,
        expanded:
            preferences.sidebarExpanded ??
            constraints.maxWidth +
                    LibraryWorkbenchMetrics.railWidth +
                    LibraryWorkbenchMetrics.contentInset >=
                940,
        sidebarWidth: preferences.sidebarWidth.clamp(
          ameMinimumSidebarWidth,
          ameMaximumSidebarWidth,
        ),
        gallery: child,
        sidebar: Material(
          type: MaterialType.transparency,
          child: ListView(
            padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
            children: [
              for (final (value, label, icon) in _destinations)
                ListTile(
                  key: ValueKey("duplicate-view-${value.name}"),
                  contentPadding: const EdgeInsets.only(left: 16, right: 4),
                  minLeadingWidth: 24,
                  horizontalTitleGap: 12,
                  leading: SizedBox(width: 24, child: Icon(icon, size: 24)),
                  title: Text(
                    label,
                    style: TextStyle(
                      fontWeight: view == value
                          ? ameFontWeightSemibold
                          : ameFontWeightRegular,
                    ),
                  ),
                  selected: view == value,
                  selectedTileColor: surfaces.selection,
                  selectedColor: surfaces.onSelection,
                  shape: LibraryWorkbenchMetrics.sidebarRowShape,
                  minTileHeight: LibraryWorkbenchMetrics.sidebarRowHeight,
                  onTap: () => onViewChanged(value),
                ),
            ],
          ),
        ),
      );
    },
  );
}
