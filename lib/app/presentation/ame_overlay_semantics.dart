import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";

import "ame_runtime_platform.dart";

// Flutter 3.44 can merge sibling OverlayPortal traversal parents and serialize
// an orphan node on Windows. Windows tooltips avoid that portal below; this
// boundary remains for other bounded anchors until flutter/flutter#190344 and
// #190431 reach the pinned SDK.
class AmeOverlayTraversalBoundary extends StatelessWidget {
  const AmeOverlayTraversalBoundary({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Semantics(container: true, explicitChildNodes: true, child: child);
  }
}

class AmeTooltip extends StatelessWidget {
  const AmeTooltip({
    required this.message,
    required this.child,
    this.waitDuration,
    super.key,
  });

  final String message;
  final Widget child;
  final Duration? waitDuration;

  @override
  Widget build(BuildContext context) {
    if (isAmeWindowsHost || defaultTargetPlatform == TargetPlatform.windows) {
      return _AmeWindowsTooltip(
        message: message,
        waitDuration: waitDuration,
        child: child,
      );
    }
    return AmeOverlayTraversalBoundary(
      child: Tooltip(
        message: message,
        waitDuration: waitDuration,
        child: child,
      ),
    );
  }
}

class _AmeWindowsTooltip extends StatefulWidget {
  const _AmeWindowsTooltip({
    required this.message,
    required this.child,
    this.waitDuration,
  });

  final String message;
  final Widget child;
  final Duration? waitDuration;

  @override
  State<_AmeWindowsTooltip> createState() => _AmeWindowsTooltipState();
}

class _AmeWindowsTooltipState extends State<_AmeWindowsTooltip>
    with SingleTickerProviderStateMixin {
  static const _animationDuration = Duration(milliseconds: 150);
  static const _defaultWaitDuration = Duration(milliseconds: 500);
  static const _defaultExitDuration = Duration(milliseconds: 100);

  Timer? _showTimer;
  Timer? _hideTimer;
  OverlayEntry? _entry;
  bool _isPointerInside = false;
  bool _hasFocus = false;
  bool _isLongPressActive = false;
  late final AnimationController _animationController;
  late final CurvedAnimation _opacityAnimation;

  @override
  void initState() {
    super.initState();
    _animationController = AnimationController(
      duration: _animationDuration,
      reverseDuration: _animationDuration,
      vsync: this,
    );
    _opacityAnimation = CurvedAnimation(
      parent: _animationController,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
  }

  @override
  void didUpdateWidget(covariant _AmeWindowsTooltip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.message != widget.message) {
      _entry?.markNeedsBuild();
    }
  }

  void _handleEnter(PointerEnterEvent event) {
    _isPointerInside = true;
    _scheduleShow();
  }

  void _handleExit(PointerExitEvent event) {
    _isPointerInside = false;
    _scheduleHide();
  }

  void _handleFocusChange(bool hasFocus) {
    _hasFocus = hasFocus;
    if (hasFocus) {
      _showImmediately();
    } else {
      _scheduleHide();
    }
  }

  void _handleLongPressStart(LongPressStartDetails details) {
    _isLongPressActive = true;
    _showImmediately();
  }

  void _handleLongPressEnd(LongPressEndDetails details) {
    _isLongPressActive = false;
    _scheduleHide();
  }

  void _handleLongPressCancel() {
    _isLongPressActive = false;
    _scheduleHide();
  }

  void _scheduleShow() {
    _hideTimer?.cancel();
    _hideTimer = null;
    if (_entry != null) {
      _animationController.forward();
      return;
    }
    _showTimer?.cancel();
    final tooltipTheme = TooltipTheme.of(context);
    _showTimer = Timer(
      widget.waitDuration ?? tooltipTheme.waitDuration ?? _defaultWaitDuration,
      _show,
    );
  }

  void _showImmediately() {
    _showTimer?.cancel();
    _showTimer = null;
    _hideTimer?.cancel();
    _hideTimer = null;
    if (_entry != null) {
      _animationController.forward();
      return;
    }
    _show();
  }

  void _scheduleHide() {
    if (_isPointerInside || _hasFocus || _isLongPressActive) {
      return;
    }
    _showTimer?.cancel();
    _showTimer = null;
    if (_entry == null) {
      return;
    }
    _hideTimer?.cancel();
    _hideTimer = Timer(
      TooltipTheme.of(context).exitDuration ?? _defaultExitDuration,
      _hide,
    );
  }

  void _show() {
    _showTimer = null;
    if (!mounted || _entry != null) {
      return;
    }
    final overlay = Overlay.maybeOf(context, rootOverlay: true);
    final targetBox = context.findRenderObject();
    final overlayBox = overlay?.context.findRenderObject();
    if (overlay == null ||
        targetBox is! RenderBox ||
        overlayBox is! RenderBox ||
        !targetBox.attached ||
        !overlayBox.attached) {
      return;
    }

    final tooltipTheme = TooltipTheme.of(context);
    final theme = Theme.of(context);
    final target = targetBox.localToGlobal(
      targetBox.size.center(Offset.zero),
      ancestor: overlayBox,
    );
    final targetSize = targetBox.size;
    final decoration =
        tooltipTheme.decoration ??
        BoxDecoration(
          color: theme.colorScheme.inverseSurface,
          borderRadius: BorderRadius.circular(4),
        );
    final textStyle =
        tooltipTheme.textStyle ??
        theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onInverseSurface,
        );
    final padding =
        tooltipTheme.padding ??
        const EdgeInsets.symmetric(horizontal: 8, vertical: 4);
    final constraints =
        tooltipTheme.constraints ?? const BoxConstraints(minHeight: 24);
    final entry = OverlayEntry(
      builder: (overlayContext) => Positioned.fill(
        bottom: MediaQuery.maybeViewInsetsOf(overlayContext)?.bottom ?? 0,
        child: IgnorePointer(
          child: ExcludeSemantics(
            child: CustomSingleChildLayout(
              delegate: _AmeWindowsTooltipPositionDelegate(
                target: target,
                targetSize: targetSize,
                verticalOffset: tooltipTheme.verticalOffset ?? 8,
                preferBelow: tooltipTheme.preferBelow ?? true,
              ),
              child: FadeTransition(
                opacity: _opacityAnimation,
                child: ConstrainedBox(
                  constraints: constraints,
                  child: DecoratedBox(
                    decoration: decoration,
                    child: Padding(
                      padding: padding,
                      child: Text(widget.message, style: textStyle),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    _entry = entry;
    overlay.insert(entry);
    _animationController.forward(from: 0);
  }

  void _hide() {
    _hideTimer = null;
    final entry = _entry;
    if (entry == null) {
      return;
    }
    _animationController.reverse().then((_) {
      if (!identical(_entry, entry)) {
        return;
      }
      entry.remove();
      _entry = null;
    });
  }

  void _removeImmediately() {
    _showTimer?.cancel();
    _hideTimer?.cancel();
    _showTimer = null;
    _hideTimer = null;
    _entry?.remove();
    _entry = null;
    _animationController.stop();
  }

  @override
  void dispose() {
    _removeImmediately();
    _opacityAnimation.dispose();
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      tooltip: widget.message,
      child: Focus(
        canRequestFocus: false,
        skipTraversal: true,
        onFocusChange: _handleFocusChange,
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          excludeFromSemantics: true,
          onLongPressStart: _handleLongPressStart,
          onLongPressEnd: _handleLongPressEnd,
          onLongPressCancel: _handleLongPressCancel,
          child: MouseRegion(
            onEnter: _handleEnter,
            onExit: _handleExit,
            child: widget.child,
          ),
        ),
      ),
    );
  }
}

class _AmeWindowsTooltipPositionDelegate extends SingleChildLayoutDelegate {
  const _AmeWindowsTooltipPositionDelegate({
    required this.target,
    required this.targetSize,
    required this.verticalOffset,
    required this.preferBelow,
  });

  static const _screenMargin = 8.0;

  final Offset target;
  final Size targetSize;
  final double verticalOffset;
  final bool preferBelow;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return BoxConstraints(
      maxWidth: (constraints.maxWidth - (_screenMargin * 2)).clamp(
        0,
        double.infinity,
      ),
      maxHeight: (constraints.maxHeight - (_screenMargin * 2)).clamp(
        0,
        double.infinity,
      ),
    );
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    final below = target.dy + (targetSize.height / 2) + verticalOffset;
    final above =
        target.dy - (targetSize.height / 2) - verticalOffset - childSize.height;
    final fitsBelow = below + childSize.height <= size.height - _screenMargin;
    final fitsAbove = above >= _screenMargin;
    final useBelow = preferBelow
        ? fitsBelow || !fitsAbove
        : !fitsAbove && fitsBelow;
    final left = (target.dx - (childSize.width / 2)).clamp(
      _screenMargin,
      (size.width - childSize.width - _screenMargin).clamp(
        _screenMargin,
        double.infinity,
      ),
    );
    final top = (useBelow ? below : above).clamp(
      _screenMargin,
      (size.height - childSize.height - _screenMargin).clamp(
        _screenMargin,
        double.infinity,
      ),
    );
    return Offset(left, top);
  }

  @override
  bool shouldRelayout(_AmeWindowsTooltipPositionDelegate oldDelegate) {
    return target != oldDelegate.target ||
        targetSize != oldDelegate.targetSize ||
        verticalOffset != oldDelegate.verticalOffset ||
        preferBelow != oldDelegate.preferBelow;
  }
}
