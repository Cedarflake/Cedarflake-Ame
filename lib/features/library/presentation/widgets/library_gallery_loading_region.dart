import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_progress.dart";

class LibraryGalleryLoadingRegion extends StatelessWidget {
  const LibraryGalleryLoadingRegion({
    required this.isLoading,
    required this.child,
    this.delay = ameProgressDelay,
    super.key,
  });

  final bool isLoading;
  final Widget child;
  final Duration delay;

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    children: [
      child,
      Positioned(
        top: 0,
        left: 0,
        right: 0,
        child: AmeLinearProgress(
          key: const Key("library-top-loading"),
          active: isLoading,
          delay: delay,
          minHeight: 2,
          semanticsLabel: "正在加载图片",
        ),
      ),
    ],
  );
}
