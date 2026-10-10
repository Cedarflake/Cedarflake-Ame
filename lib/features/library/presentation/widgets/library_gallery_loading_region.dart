import "package:flutter/material.dart";

class LibraryGalleryLoadingRegion extends StatelessWidget {
  const LibraryGalleryLoadingRegion({
    required this.isLoading,
    required this.child,
    super.key,
  });

  final bool isLoading;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        child,
        if (isLoading)
          const Positioned(
            top: 0,
            left: 0,
            right: 0,
            child: IgnorePointer(
              child: LinearProgressIndicator(
                key: Key("library-top-loading"),
                minHeight: 2,
                semanticsLabel: "正在加载图片",
              ),
            ),
          ),
      ],
    );
  }
}
