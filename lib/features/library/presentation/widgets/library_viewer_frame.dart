import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../../../app/window/ame_window_fullscreen.dart";
import "library_viewer_controls.dart";
import "library_viewer_transition.dart";

class LibraryViewerFrame extends ConsumerStatefulWidget {
  const LibraryViewerFrame({
    required this.title,
    required this.onBack,
    required this.builder,
    this.backLabel,
    this.actions,
    super.key,
  });

  final Widget title;
  final VoidCallback onBack;
  final String? backLabel;
  final Widget? actions;
  final Widget Function(
    BuildContext context,
    bool isFullscreen,
    VoidCallback? toggleFullscreen,
    VoidCallback goBack,
  )
  builder;

  @override
  ConsumerState<LibraryViewerFrame> createState() => _LibraryViewerFrameState();
}

class _LibraryViewerFrameState extends ConsumerState<LibraryViewerFrame> {
  final _actionsKey = GlobalKey(debugLabel: "viewer-organization-actions");
  AmeWindowFullscreen? _fullscreen;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!LibraryViewerActivity.isActiveOf(context)) {
      unawaited(_setFullscreen(false));
    }
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  Future<void> _setFullscreen(bool enabled) async {
    try {
      await _fullscreen?.setEnabled(enabled);
    } on Object {
      if (mounted) {
        showAmeTransientFeedback(
          context,
          message: "无法切换全屏，请重试",
          severity: AmeNotificationSeverity.error,
        );
      }
    }
  }

  Future<void> _back() async {
    await _setFullscreen(false);
    if (mounted) widget.onBack();
  }

  void _escape() {
    if (_fullscreen?.isFullscreen ?? false) {
      unawaited(_setFullscreen(false));
    } else {
      unawaited(_back());
    }
  }

  void _toggleFullscreen() =>
      unawaited(_setFullscreen(!(_fullscreen?.isFullscreen ?? false)));

  @override
  void dispose() {
    _fullscreen?.removeListener(_changed);
    unawaited(_setFullscreen(false));
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final fullscreen = ref.watch(ameWindowFullscreenProvider);
    if (_fullscreen != fullscreen) {
      _fullscreen?.removeListener(_changed);
      _fullscreen = fullscreen;
      _fullscreen?.addListener(_changed);
    }
    final isFullscreen = _fullscreen?.isFullscreen ?? false;
    final actions = widget.actions == null
        ? null
        : KeyedSubtree(key: _actionsKey, child: widget.actions!);
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): _escape,
        const SingleActivator(LogicalKeyboardKey.backspace): _back,
        const SingleActivator(LogicalKeyboardKey.browserBack): _back,
        const SingleActivator(LogicalKeyboardKey.f11): _toggleFullscreen,
      },
      child: Column(
        children: [
          if (!isFullscreen) ...[
            LibraryViewerTopBar(
              title: widget.title,
              onBack: () => unawaited(_back()),
              backLabel: widget.backLabel,
              organizationActions: actions,
            ),
            const Divider(height: 1),
          ],
          Expanded(
            child: Stack(
              fit: StackFit.expand,
              children: [
                widget.builder(
                  context,
                  isFullscreen,
                  fullscreen == null || fullscreen.isChanging
                      ? null
                      : _toggleFullscreen,
                  () => unawaited(_back()),
                ),
                if (isFullscreen && actions != null)
                  Positioned(
                    top: 12,
                    right: 20,
                    child: Material(
                      color: Theme.of(context).colorScheme.surfaceContainerLow,
                      shape: const StadiumBorder(),
                      child: actions,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
