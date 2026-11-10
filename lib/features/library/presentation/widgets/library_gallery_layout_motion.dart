import "package:material_ui/material_ui.dart";
import "package:flutter/rendering.dart";

import "../../../../app/presentation/ame_motion.dart";
import "../coordination/library_gallery_motion_publication.dart";
import "library_gallery_exit_visuals.dart";
import "library_sidebar_content.dart";

/// Keeps only mounted sliver items, so reflow motion is independent of library size.
class LibraryGalleryLayoutMotion extends StatefulWidget {
  const LibraryGalleryLayoutMotion({
    required this.queryIdentity,
    required this.geometryIdentity,
    required this.scrollController,
    this.duplicatePublication,
    this.contentOffset = Offset.zero,
    this.scrollOffsetCorrection = 0,
    required this.child,
    super.key,
  });

  final Object queryIdentity;
  final Object geometryIdentity;
  final ScrollController scrollController;
  final LibraryGalleryMotionPublication? duplicatePublication;
  final Offset contentOffset;
  final double scrollOffsetCorrection;
  final Widget child;

  @override
  State<LibraryGalleryLayoutMotion> createState() => _LayoutMotionState();
}

class _LayoutMotionState extends State<LibraryGalleryLayoutMotion>
    with TickerProviderStateMixin {
  late final AnimationController _animation = AnimationController(
    vsync: this,
    value: 1,
  );
  late final AnimationController _effects = AnimationController(
    vsync: this,
    value: 1,
  )..addStatusListener(_releaseExitVisuals);
  final _exits = LibraryGalleryExitVisuals();
  final _items = <Object, _ItemMotion>{};
  final _present = <Object>{};
  var _publicationGeneration = 0;
  bool _duplicateTransition = false;
  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;
  double? _pendingScrollOffset;
  bool _isReleaseScheduled = false;

  double get _scrollOffset => widget.scrollController.hasClients
      ? widget.scrollController.position.pixels
      : 0;
  double get _progress => _animation.value;

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
    if (_skipMotion) _finishMotion();
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryLayoutMotion oldWidget) {
    super.didUpdateWidget(oldWidget);
    final publication = widget.duplicatePublication;
    final previous = oldWidget.duplicatePublication;
    if (widget.scrollController != oldWidget.scrollController) {
      _items.clear();
      _pendingScrollOffset = null;
      _finishMotion();
    } else if (widget.queryIdentity != oldWidget.queryIdentity) {
      if (publication != null &&
          previous != null &&
          publication.changesDuplicatesFrom(previous)) {
        _beginMotion(duplicates: true);
      } else if (!(_duplicateTransition &&
          (_animation.isAnimating || _effects.isAnimating) &&
          publication != null &&
          previous != null &&
          publication.keepsDuplicatePresentation(previous))) {
        _items.clear();
        _pendingScrollOffset = null;
        _finishMotion();
      }
    } else if (widget.geometryIdentity != oldWidget.geometryIdentity) {
      _beginMotion(duplicates: false);
    }
    _releaseScrollCorrectionAfterLayout();
  }

  void _beginMotion({required bool duplicates}) {
    if (_skipMotion || _items.isEmpty) {
      _finishMotion();
      return;
    }
    for (final item in _items.values) {
      item
        ..begin = _displayRect(item)
        ..appearing = false;
    }
    _duplicateTransition = duplicates;
    if (duplicates) {
      final renderObject = context.findRenderObject();
      final viewport = renderObject is RenderBox && renderObject.hasSize
          ? Offset.zero & renderObject.size
          : Rect.zero;
      _exits.capture([
        for (final entry in _items.entries)
          if (entry.value.boundary case final boundary?)
            (identity: entry.key, boundary: boundary, rect: entry.value.begin),
      ], viewport.shift(-widget.contentOffset));
      _present.clear();
      final generation = ++_publicationGeneration;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || generation != _publicationGeneration) return;
        setState(() => _exits.removePresent(_present));
      });
      if (!_effects.isAnimating) {
        _effects.value = 0;
        AmeMotion.fastEffects.animate(_effects, forward: true);
      }
    } else {
      _effects.value = 1;
      _exits.clear();
    }
    // The sliver applies this correction after building its children.
    _pendingScrollOffset = _scrollOffset + widget.scrollOffsetCorrection;
    _animation.value = 0;
    AmeMotion.spatial.animate(_animation, forward: true);
  }

  void _finishMotion() {
    _duplicateTransition = false;
    _publicationGeneration++;
    _exits.clear();
    _animation.value = 1;
    _effects.value = 1;
  }

  void _releaseExitVisuals(AnimationStatus status) {
    if (status == AnimationStatus.completed && !_exits.isEmpty) {
      setState(_exits.clear);
    }
  }

  Rect _displayRect(_ItemMotion item) => Rect.lerp(
    item.begin,
    item.target.shift(Offset(0, -(_pendingScrollOffset ?? _scrollOffset))),
    _progress,
  )!;

  _ItemMotion _register(
    Object identity,
    Object owner,
    Rect target,
    GlobalKey? boundary,
  ) {
    _present.add(identity);
    final item = _items.putIfAbsent(
      identity,
      () => _ItemMotion(
        target.shift(Offset(0, -(_pendingScrollOffset ?? _scrollOffset))),
        target,
        owner,
        appearing:
            boundary != null && _duplicateTransition && _effects.isAnimating,
      ),
    );
    item
      ..target = target
      ..owner = owner
      ..boundary = boundary;
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
    _effects.dispose();
    _exits.clear();
    _items.clear();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _MotionScope(
    state: this,
    geometryIdentity: (widget.geometryIdentity, widget.queryIdentity),
    child: NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        // Layout corrections can end wheel activity while the viewport lays out.
        // Only a new user scroll interrupts reflow motion.
        final userScroll =
            notification is UserScrollNotification &&
            notification.direction != ScrollDirection.idle;
        final moved =
            notification is ScrollUpdateNotification &&
            _pendingScrollOffset == null &&
            notification.scrollDelta != 0;
        if (notification.depth == 0 &&
            (userScroll || moved) &&
            (_animation.isAnimating || _effects.isAnimating)) {
          setState(_finishMotion);
        }
        return false;
      },
      child: LibrarySidebarContent(
        child: Stack(
          fit: StackFit.expand,
          children: [
            widget.child,
            if (!_exits.isEmpty)
              Positioned.fill(
                child: _exits.build(
                  ReverseAnimation(_effects),
                  widget.contentOffset,
                ),
              ),
          ],
        ),
      ),
    ),
  );
}

class _ItemMotion {
  _ItemMotion(this.begin, this.target, this.owner, {required this.appearing});

  Rect begin;
  Rect target;
  Object owner;
  GlobalKey? boundary;
  bool appearing;
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
    this.retainExit = false,
    super.key,
  });

  final Object identity;
  final Rect rect;
  final Widget Function(BuildContext context, Size size) builder;
  final bool retainExit;

  @override
  State<LibraryGalleryMotionItem> createState() => _MotionItemState();
}

class _MotionItemState extends State<LibraryGalleryMotionItem> {
  _LayoutMotionState? _scope;
  final _owner = Object();
  final _boundary = GlobalKey();

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
    final item = scope._register(
      widget.identity,
      _owner,
      widget.rect,
      widget.retainExit ? _boundary : null,
    );
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
              child: widget.retainExit
                  ? FadeTransition(
                      opacity: item.appearing
                          ? scope._effects
                          : const AlwaysStoppedAnimation(1),
                      child: LibraryGalleryPaintBoundary(
                        key: _boundary,
                        child: widget.builder(context, rect.size),
                      ),
                    )
                  : widget.builder(context, rect.size),
            ),
          );
        },
      ),
    );
  }
}
