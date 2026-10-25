import "dart:async";

import "package:flutter/material.dart";

class LibraryGalleryLoadingRegion extends StatefulWidget {
  const LibraryGalleryLoadingRegion({
    required this.isLoading,
    required this.child,
    this.delay = Duration.zero,
    super.key,
  });

  final bool isLoading;
  final Widget child;
  final Duration delay;

  @override
  State<LibraryGalleryLoadingRegion> createState() =>
      _LibraryGalleryLoadingRegionState();
}

class _LibraryGalleryLoadingRegionState
    extends State<LibraryGalleryLoadingRegion> {
  Timer? _timer;
  bool _showLoading = false;

  @override
  void initState() {
    super.initState();
    _updateLoading();
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryLoadingRegion oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isLoading != widget.isLoading ||
        oldWidget.delay != widget.delay) {
      _updateLoading();
    }
  }

  void _updateLoading() {
    _timer?.cancel();
    _showLoading = widget.isLoading && widget.delay == Duration.zero;
    if (widget.isLoading && !_showLoading) {
      _timer = Timer(widget.delay, () => setState(() => _showLoading = true));
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        widget.child,
        if (_showLoading)
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
