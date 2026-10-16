import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../application/library_workbench_controller.dart";
import "../library_strings.dart";
import "../library_workbench_strings.dart";
import "library_main_surface.dart";
import "library_navigation_resize_handle.dart";
import "library_sidebar_transition.dart";

class LibraryWorkbench extends ConsumerStatefulWidget {
  const LibraryWorkbench({
    required this.navigationBuilder,
    required this.galleryBuilder,
    required this.duplicatesBuilder,
    required this.settingsBuilder,
    required this.onSidebarResizeChanged,
    required this.onPreferenceError,
    super.key,
  });

  final Widget Function(double width) navigationBuilder;
  final WidgetBuilder galleryBuilder;
  final WidgetBuilder duplicatesBuilder;
  final WidgetBuilder settingsBuilder;
  final ValueChanged<bool> onSidebarResizeChanged;
  final void Function(String title, Object error) onPreferenceError;

  @override
  ConsumerState<LibraryWorkbench> createState() => _LibraryWorkbenchState();
}

class _LibraryWorkbenchState extends ConsumerState<LibraryWorkbench> {
  static const railWidth = 64.0;
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
        widget.onPreferenceError(
          LibraryWorkbenchStrings.saveSidebarFailed,
          error,
        );
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
              ? LibraryWorkbenchStrings.collapseSidebar
              : LibraryWorkbenchStrings.expandSidebar,
          child: IconButton(
            key: const Key("library-sidebar-toggle"),
            onPressed: () => unawaited(_persistSidebar(expanded: !expanded)),
            icon: Icon(
              expanded
                  ? Symbols.left_panel_close_rounded
                  : Symbols.left_panel_open_rounded,
            ),
          ),
        );
        final gallery = LibrarySidebarTransition(
          expanded: expanded,
          sidebarWidth: sidebarWidth,
          sidebar: widget.navigationBuilder(sidebarWidth),
          gallery: LibraryMainSurface(child: widget.galleryBuilder(context)),
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
        );
        return Row(
          children: [
            _FunctionRail(workspace: workspace, sidebarToggle: toggle),
            Expanded(
              child: Stack(
                fit: StackFit.expand,
                children: [
                  _retainedWorkspace(
                    workspace: LibraryWorkspace.gallery,
                    visible: workspace == LibraryWorkspace.gallery,
                    child: gallery,
                  ),
                  if (_visited.contains(LibraryWorkspace.duplicates))
                    _retainedWorkspace(
                      workspace: LibraryWorkspace.duplicates,
                      visible: workspace == LibraryWorkspace.duplicates,
                      child: LibraryMainSurface(
                        child: widget.duplicatesBuilder(context),
                      ),
                    ),
                  if (_visited.contains(LibraryWorkspace.settings))
                    _retainedWorkspace(
                      workspace: LibraryWorkspace.settings,
                      visible: workspace == LibraryWorkspace.settings,
                      child: LibraryMainSurface(
                        child: widget.settingsBuilder(context),
                      ),
                    ),
                ],
              ),
            ),
          ],
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
  const _FunctionRail({required this.workspace, required this.sidebarToggle});
  final LibraryWorkspace workspace;
  final Widget sidebarToggle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(libraryWorkbenchControllerProvider.notifier);
    return NavigationRail(
      key: const Key("library-function-rail"),
      minWidth: _LibraryWorkbenchState.railWidth,
      backgroundColor: Theme.of(context).colorScheme.surfaceContainerLow,
      labelType: NavigationRailLabelType.all,
      selectedIndex: switch (workspace) {
        LibraryWorkspace.gallery => 0,
        LibraryWorkspace.duplicates => 1,
        LibraryWorkspace.settings => null,
      },
      onDestinationSelected: (index) => controller.select(
        index == 0 ? LibraryWorkspace.gallery : LibraryWorkspace.duplicates,
      ),
      destinations: const [
        NavigationRailDestination(
          icon: Icon(
            Symbols.photo_library_rounded,
            key: Key("workbench-gallery"),
          ),
          label: Text(LibraryStrings.library),
        ),
        NavigationRailDestination(
          icon: Icon(Symbols.stacks_rounded, key: Key("workbench-duplicates")),
          label: Text(LibraryWorkbenchStrings.duplicates),
        ),
      ],
      trailingAtBottom: true,
      trailing: Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (workspace == LibraryWorkspace.gallery) sidebarToggle,
            AmeTooltip(
              message: LibraryStrings.settings,
              child: IconButton(
                key: const Key("library-sidebar-settings"),
                isSelected: workspace == LibraryWorkspace.settings,
                onPressed: () => controller.select(LibraryWorkspace.settings),
                icon: const Icon(Symbols.settings_rounded),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
