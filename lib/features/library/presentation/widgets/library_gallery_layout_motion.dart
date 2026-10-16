import "package:flutter/material.dart";

import "library_sidebar_content.dart";

/// Keeps only mounted sliver items, so reflow motion is independent of library size.
class LibraryGalleryLayoutMotion extends StatefulWidget {
  const LibraryGalleryLayoutMotion({
    required this.queryIdentity,
    required this.geometryIdentity,
    required this.scrollController,
    this.scrollOffsetCorrection = 0,
    required this.child,
    super.key,
  });

  final Object queryIdentity;
  final Object geometryIdentity;
  final ScrollController scrollController;
  final double scrollOffsetCorrection;
  final Widget child;

  @override
  State<LibraryGalleryLayoutMotion> createState() => _LayoutMotionState();
}

class _LayoutMotionState extends State<LibraryGalleryLayoutMotion>
    with SingleTickerProviderStateMixin {
  late final AnimationController _animation = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 180),
    value: 1,
  );
  final _items = <Object, _ItemMotion>{};
  bool _skipMotion = false;
  double? _pendingScrollOffset;
  bool _isReleaseScheduled = false;

  double get _scrollOffset => widget.scrollController.hasClients
      ? widget.scrollController.position.pixels
      : 0;
  double get _progress => Curves.easeInOutCubic.transform(_animation.value);

  void _releaseScrollCorrectionAfterLayout() {
    if (_isReleaseScheduled) return;
    _isReleaseScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _isReleaseScheduled = false;
      if (mounted) _pendingScrollOffset = null;
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _skipMotion =
        MediaQuery.disableAnimationsOf(context) ||
        !TickerMode.valuesOf(context).enabled;
    if (_skipMotion) _animation.value = 1;
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryLayoutMotion oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.queryIdentity != oldWidget.queryIdentity ||
        widget.scrollController != oldWidget.scrollController) {
      _items.clear();
      _pendingScrollOffset = null;
      _animation.value = 1;
    } else if (widget.geometryIdentity != oldWidget.geometryIdentity) {
      for (final item in _items.values) {
        item.begin = _displayRect(item);
      }
      // The sliver applies this correction after building its children.
      // Use the destination offset now so the first painted frame is continuous.
      _pendingScrollOffset = _scrollOffset + widget.scrollOffsetCorrection;
      if (_skipMotion || _items.isEmpty) {
        _animation.value = 1;
      } else {
        _animation.forward(from: 0);
      }
    }
    _releaseScrollCorrectionAfterLayout();
  }

  Rect _displayRect(_ItemMotion item) => Rect.lerp(
    item.begin,
    item.target.shift(Offset(0, -(_pendingScrollOffset ?? _scrollOffset))),
    _progress,
  )!;

  _ItemMotion _register(Object identity, Object owner, Rect target) {
    final item = _items.putIfAbsent(
      identity,
      () => _ItemMotion(
        target.shift(Offset(0, -(_pendingScrollOffset ?? _scrollOffset))),
        target,
        owner,
      ),
    );
    item
      ..target = target
      ..owner = owner;
    return item;
  }

  void _retire(Object identity, Object owner) {
    // A tile can move to another lazy row in the same frame. Its replacement
    // inherits the current rectangle before the old row releases its lease.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (identical(_items[identity]?.owner, owner)) _items.remove(identity);
    });
  }

  @override
  void dispose() {
    _animation.dispose();
    _items.clear();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _MotionScope(
    state: this,
    geometryIdentity: widget.geometryIdentity,
    child: NotificationListener<UserScrollNotification>(
      onNotification: (_) {
        _animation.value = 1;
        return false;
      },
      child: LibrarySidebarContent(child: widget.child),
    ),
  );
}

class _ItemMotion {
  _ItemMotion(this.begin, this.target, this.owner);

  Rect begin;
  Rect target;
  Object owner;
}

class _MotionScope extends InheritedWidget {
  const _MotionScope({
    required this.state,
    required this.geometryIdentity,
    required super.child,
  });

  final _LayoutMotionState state;
  final Object geometryIdentity;

  @override
  bool updateShouldNotify(_MotionScope oldWidget) =>
      state != oldWidget.state ||
      geometryIdentity != oldWidget.geometryIdentity;
}

class LibraryGalleryMotionItem extends StatefulWidget {
  const LibraryGalleryMotionItem({
    required this.identity,
    required this.rect,
    required this.builder,
    super.key,
  });

  final Object identity;
  final Rect rect;
  final Widget Function(BuildContext context, Size size) builder;

  @override
  State<LibraryGalleryMotionItem> createState() => _MotionItemState();
}

class _MotionItemState extends State<LibraryGalleryMotionItem> {
  _LayoutMotionState? _scope;
  final _owner = Object();

  @override
  void dispose() {
    _scope?._retire(widget.identity, _owner);
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryMotionItem oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.identity != widget.identity) {
      _scope?._retire(oldWidget.identity, _owner);
    }
  }

  @override
  Widget build(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_MotionScope>()
        ?.state;
    if (_scope != scope) _scope?._retire(widget.identity, _owner);
    _scope = scope;
    if (scope == null) return widget.builder(context, widget.rect.size);
    final item = scope._register(widget.identity, _owner, widget.rect);
    return SizedBox.fromSize(
      size: widget.rect.size,
      child: AnimatedBuilder(
        animation: scope._animation,
        builder: (context, _) {
          final rect = scope._displayRect(item);
          final target = widget.rect.shift(
            Offset(0, -(scope._pendingScrollOffset ?? scope._scrollOffset)),
          );
          return Transform.translate(
            offset: rect.topLeft - target.topLeft,
            child: OverflowBox(
              alignment: Alignment.topLeft,
              minWidth: rect.width,
              maxWidth: rect.width,
              minHeight: rect.height,
              maxHeight: rect.height,
              child: widget.builder(context, rect.size),
            ),
          );
        },
      ),
    );
  }
}
