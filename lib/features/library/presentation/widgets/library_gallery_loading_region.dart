import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_loading_indicator.dart";

class LibraryGalleryLoadingRegion extends StatefulWidget {
  const LibraryGalleryLoadingRegion({
    required this.isLoading,
    required this.child,
    this.showContentIndicator = false,
    this.delay = ameProgressDelay,
    super.key,
  });

  final bool isLoading;
  final Widget child;
  final bool showContentIndicator;
  final Duration delay;

  static bool showsContentIndicatorOf(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<_ContentLoadingPriority>()
          ?.active ??
      false;

  @override
  State<LibraryGalleryLoadingRegion> createState() =>
      _LibraryGalleryLoadingRegionState();
}

class _LibraryGalleryLoadingRegionState
    extends State<LibraryGalleryLoadingRegion> {
  late bool _center = widget.showContentIndicator;

  @override
  void didUpdateWidget(LibraryGalleryLoadingRegion oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Retain the departing indicator while the shared loading layer fades out.
    if (widget.isLoading) _center = widget.showContentIndicator;
  }

  @override
  Widget build(BuildContext context) => _ContentLoadingPriority(
    active: widget.isLoading && widget.showContentIndicator,
    child: Stack(
      fit: StackFit.expand,
      children: [
        widget.child,
        Positioned.fill(
          child: AmeProgressVisibility(
            key: const Key("library-loading-feedback"),
            active: widget.isLoading,
            delay: widget.delay,
            child: Stack(
              fit: StackFit.expand,
              children: [
                Positioned(
                  top: 0,
                  left: 0,
                  right: 0,
                  child: ExcludeSemantics(
                    excluding: _center,
                    child: LinearProgressIndicator(
                      key: Key("library-top-loading"),
                      minHeight: 2,
                      semanticsLabel: context.l10n.loadingPhotosShort,
                    ),
                  ),
                ),
                if (_center)
                  Center(
                    child: AmeLoadingIndicator(
                      semanticsLabel: context.l10n.loadingPhotosShort,
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

class _ContentLoadingPriority extends InheritedWidget {
  const _ContentLoadingPriority({required this.active, required super.child});
  final bool active;

  @override
  bool updateShouldNotify(_ContentLoadingPriority oldWidget) =>
      active != oldWidget.active;
}
