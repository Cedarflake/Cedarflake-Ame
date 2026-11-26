import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../settings/domain/ame_preferences.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../application/library_workbench_controller.dart";
import "library_navigation_resize_handle.dart";
import "library_sidebar_content.dart";
import "library_sidebar_transition.dart";
import "library_workbench_metrics.dart";
import "library_workbench_frame.dart";
import "library_workspace_width_transition.dart";

class LibraryWorkbench extends ConsumerStatefulWidget {
  const LibraryWorkbench({
    required this.navigationBuilder,
    required this.albumsNavigationBuilder,
    required this.galleryBuilder,
    required this.duplicatesBuilder,
    required this.settingsBuilder,
    required this.onSidebarResizeChanged,
    required this.onPreferenceError,
    this.onWorkspaceSelected,
    this.albumLanding,
    required this.globalBarBuilder,
    super.key,
  });

  final Widget Function(double width) navigationBuilder;
  final Widget Function(double width) albumsNavigationBuilder;
  final WidgetBuilder galleryBuilder;
  final WidgetBuilder duplicatesBuilder;
  final WidgetBuilder settingsBuilder;
  final ValueChanged<bool> onSidebarResizeChanged;
  final void Function(AmeMessage title, Object error) onPreferenceError;
  final ValueChanged<LibraryWorkspace>? onWorkspaceSelected;
  final Widget? albumLanding;
  final Widget Function(double sidebarWidth) globalBarBuilder;

  @override
  ConsumerState<LibraryWorkbench> createState() => _LibraryWorkbenchState();
}

class _LibraryWorkbenchState extends ConsumerState<LibraryWorkbench> {
  final Set<LibraryWorkspace> _visited = {LibraryWorkspace.gallery};
  late double _sidebarWidth;

  @override
  void initState() {
    super.initState();
    _sidebarWidth = ref.read(amePreferencesControllerProvider).sidebarWidth;
  }

  Future<void> _persistSidebar({bool? expanded, double? width}) async {
    final preferences = ref.read(amePreferencesControllerProvider);
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(
            preferences.copyWith(
              sidebarExpanded: expanded,
              sidebarWidth: width,
            ),
          );
    } on Object catch (error) {
      if (mounted) {
        setState(() => _sidebarWidth = preferences.sidebarWidth);
        widget.onPreferenceError(AmeLabel.saveSidebarFailed, error);
      }
    }
  }

  void _finishResize(double width) {
    unawaited(_persistSidebar(width: width));
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        widget.onSidebarResizeChanged(false);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final workspace = ref.watch(libraryWorkbenchControllerProvider);
    final preferences = ref.watch(amePreferencesControllerProvider);
    _visited.add(workspace);
    return LayoutBuilder(
      builder: (context, constraints) {
        final expanded =
            preferences.sidebarExpanded ?? constraints.maxWidth >= 940;
        final sidebarWidth = _sidebarWidth
            .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
            .toDouble();
        final toggle = AmeTooltip(
          message: expanded
              ? context.l10n.collapseSidebar
              : context.l10n.expandSidebar,
          child: IconButton(
            key: const Key("library-sidebar-toggle"),
            onPressed:
                workspace.isBrowsing ||
                    (workspace == LibraryWorkspace.duplicates &&
                        constraints.maxWidth >= 920)
                ? () => unawaited(_persistSidebar(expanded: !expanded))
                : null,
            icon: const Icon(
              Symbols.menu_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
            ),
          ),
        );
        final rail = _FunctionRail(
          workspace: workspace,
          onSelected: widget.onWorkspaceSelected,
        );
        final destinations = [
          if (_visited.contains(LibraryWorkspace.duplicates))
            _retainedWorkspace(
              workspace: LibraryWorkspace.duplicates,
              visible: workspace == LibraryWorkspace.duplicates,
              child: widget.duplicatesBuilder(context),
            ),
          if (_visited.contains(LibraryWorkspace.settings))
            _retainedWorkspace(
              workspace: LibraryWorkspace.settings,
              visible: workspace == LibraryWorkspace.settings,
              child: LibraryWorkspaceContentPane(
                child: widget.settingsBuilder(context),
              ),
            ),
        ];
        return LibraryWorkspaceWidthTransition(
          destination: workspace,
          contentInset:
              workspace == LibraryWorkspace.settings ||
                  !expanded ||
                  (workspace == LibraryWorkspace.duplicates &&
                      constraints.maxWidth < 920)
              ? LibraryWorkbenchMetrics.contentInset
              : sidebarWidth,
          child: LibrarySidebarTransition(
            workspace: workspace.isBrowsing
                ? workspace
                : LibraryWorkspace.gallery,
            expanded: expanded,
            sidebarWidth: sidebarWidth,
            sidebar: Stack(
              fit: StackFit.expand,
              children: [
                _retainedWorkspace(
                  workspace: LibraryWorkspace.gallery,
                  visible: workspace != LibraryWorkspace.albums,
                  child: widget.navigationBuilder(sidebarWidth),
                ),
                if (_visited.contains(LibraryWorkspace.albums))
                  _retainedWorkspace(
                    workspace: LibraryWorkspace.albums,
                    visible: workspace == LibraryWorkspace.albums,
                    child: widget.albumsNavigationBuilder(sidebarWidth),
                  ),
              ],
            ),
            gallery: Stack(
              fit: StackFit.expand,
              children: [
                _retainedWorkspace(
                  workspace: LibraryWorkspace.gallery,
                  visible:
                      workspace != LibraryWorkspace.albums ||
                      widget.albumLanding == null,
                  child: widget.galleryBuilder(context),
                ),
                if (workspace == LibraryWorkspace.albums &&
                    widget.albumLanding != null)
                  LibrarySidebarContent(
                    alignment: Alignment.topCenter,
                    child: widget.albumLanding!,
                  ),
              ],
            ),
            resizeHandle: LibraryNavigationResizeHandle(
              width: sidebarWidth,
              minimumWidth: ameMinimumSidebarWidth,
              maximumWidth: ameMaximumSidebarWidth,
              defaultWidth: ameDefaultSidebarWidth,
              onWidthChangeStart: () => widget.onSidebarResizeChanged(true),
              onWidthChanged: (width) => setState(() => _sidebarWidth = width),
              onWidthChangeEnd: _finishResize,
              onWidthChangeCancel: () => widget.onSidebarResizeChanged(false),
            ),
            frameBuilder: (context, _, panels) {
              return LibraryWorkbenchFrame(
                navigation: rail,
                sidebarToggle: toggle,
                globalBar: widget.globalBarBuilder(sidebarWidth),
                content: Stack(
                  fit: StackFit.expand,
                  children: [
                    _retainedWorkspace(
                      workspace: LibraryWorkspace.gallery,
                      visible: workspace.isBrowsing,
                      child: panels,
                    ),
                    ...destinations,
                  ],
                ),
              );
            },
          ),
        );
      },
    );
  }

  Widget _retainedWorkspace({
    required LibraryWorkspace workspace,
    required bool visible,
    required Widget child,
  }) {
    return Visibility(
      key: ValueKey(workspace),
      visible: visible,
      maintainState: true,
      child: ExcludeFocus(excluding: !visible, child: child),
    );
  }
}

class _FunctionRail extends ConsumerWidget {
  const _FunctionRail({required this.workspace, this.onSelected});
  final LibraryWorkspace workspace;
  final ValueChanged<LibraryWorkspace>? onSelected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(libraryWorkbenchControllerProvider.notifier);
    final select = onSelected ?? controller.select;
    final surfaces = AmeWorkbenchSurfaces.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 20),
      child: NavigationRail(
        key: const Key("library-function-rail"),
        minWidth: LibraryWorkbenchMetrics.railWidth,
        backgroundColor: surfaces.navigation,
        indicatorColor: surfaces.selection,
        selectedIconTheme: IconThemeData(color: surfaces.onSelection),
        labelType: NavigationRailLabelType.all,
        selectedIndex: switch (workspace) {
          LibraryWorkspace.gallery => 0,
          LibraryWorkspace.albums => 1,
          LibraryWorkspace.duplicates => 2,
          LibraryWorkspace.settings => null,
        },
        onDestinationSelected: (index) => select(
          [
            LibraryWorkspace.gallery,
            LibraryWorkspace.albums,
            LibraryWorkspace.duplicates,
          ][index],
        ),
        destinations: [
          NavigationRailDestination(
            icon: Icon(
              Symbols.photo_library_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              key: Key("workbench-gallery"),
            ),
            selectedIcon: Icon(
              Symbols.photo_library_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              fill: 1,
              key: Key("workbench-gallery"),
            ),
            label: Text(context.l10n.library),
          ),
          NavigationRailDestination(
            icon: Icon(
              Symbols.photo_album_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              key: Key("workbench-albums"),
            ),
            selectedIcon: Icon(
              Symbols.photo_album_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              fill: 1,
              key: Key("workbench-albums"),
            ),
            label: Text(context.l10n.albums),
          ),
          NavigationRailDestination(
            icon: Icon(
              Symbols.stacks_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              key: Key("workbench-duplicates"),
            ),
            selectedIcon: Icon(
              Symbols.stacks_rounded,
              size: LibraryWorkbenchMetrics.iconSize,
              fill: 1,
              key: Key("workbench-duplicates"),
            ),
            label: Text(context.l10n.duplicates),
          ),
        ],
        trailingAtBottom: true,
        trailing: Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              AmeTooltip(
                message: context.l10n.settingsTitle,
                child: IconButton(
                  key: const Key("library-sidebar-settings"),
                  isSelected: workspace == LibraryWorkspace.settings,
                  style: IconButton.styleFrom(
                    fixedSize: const Size.square(48),
                    shape: const CircleBorder(),
                    backgroundColor: workspace == LibraryWorkspace.settings
                        ? surfaces.selection
                        : null,
                    foregroundColor: workspace == LibraryWorkspace.settings
                        ? surfaces.onSelection
                        : Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                  onPressed: () => select(LibraryWorkspace.settings),
                  icon: const Icon(
                    Symbols.settings_rounded,
                    size: LibraryWorkbenchMetrics.iconSize,
                  ),
                  selectedIcon: const Icon(
                    Symbols.settings_rounded,
                    size: LibraryWorkbenchMetrics.iconSize,
                    fill: 1,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
