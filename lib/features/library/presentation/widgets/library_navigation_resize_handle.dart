import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";

class LibraryNavigationResizeHandle extends StatefulWidget {
  const LibraryNavigationResizeHandle({
    required this.width,
    required this.minimumWidth,
    required this.maximumWidth,
    required this.defaultWidth,
    required this.onWidthChangeStart,
    required this.onWidthChanged,
    required this.onWidthChangeEnd,
    required this.onWidthChangeCancel,
    super.key,
  });

  final double width;
  final double minimumWidth;
  final double maximumWidth;
  final double defaultWidth;
  final VoidCallback onWidthChangeStart;
  final ValueChanged<double> onWidthChanged;
  final ValueChanged<double> onWidthChangeEnd;
  final VoidCallback onWidthChangeCancel;

  static const hitTargetWidth = 16.0;

  @override
  State<LibraryNavigationResizeHandle> createState() =>
      _LibraryNavigationResizeHandleState();
}

class _LibraryNavigationResizeHandleState
    extends State<LibraryNavigationResizeHandle> {
  static const _keyboardStep = 16.0;

  final FocusNode _focusNode = FocusNode();
  double? _dragStartWidth;
  double? _dragStartGlobalX;
  double? _pendingWidth;

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final nextWidth = _bounded(widget.width + _keyboardStep);
    final previousWidth = _bounded(widget.width - _keyboardStep);
    return Semantics(
      label: "调整侧栏宽度",
      value: "${widget.width.round()} 像素",
      increasedValue: "${nextWidth.round()} 像素",
      decreasedValue: "${previousWidth.round()} 像素",
      onIncrease: () => _setAndCommit(nextWidth),
      onDecrease: () => _setAndCommit(previousWidth),
      child: Focus(
        focusNode: _focusNode,
        onKeyEvent: _handleKeyEvent,
        child: MouseRegion(
          cursor: SystemMouseCursors.resizeLeftRight,
          child: GestureDetector(
            key: const Key("library-sidebar-resize-handle"),
            behavior: HitTestBehavior.opaque,
            dragStartBehavior: DragStartBehavior.down,
            onTap: _focusNode.requestFocus,
            onHorizontalDragStart: (details) {
              _focusNode.requestFocus();
              widget.onWidthChangeStart();
              _dragStartWidth = widget.width;
              _dragStartGlobalX = details.globalPosition.dx;
              _pendingWidth = widget.width;
            },
            onHorizontalDragUpdate: (details) {
              final startWidth = _dragStartWidth ?? widget.width;
              final startGlobalX =
                  _dragStartGlobalX ?? details.globalPosition.dx;
              final width = _bounded(
                startWidth + details.globalPosition.dx - startGlobalX,
              );
              _pendingWidth = width;
              widget.onWidthChanged(width);
            },
            onHorizontalDragEnd: (_) {
              widget.onWidthChangeEnd(_pendingWidth ?? widget.width);
              _clearDragState();
            },
            onHorizontalDragCancel: () {
              widget.onWidthChangeCancel();
              _clearDragState();
            },
            onDoubleTap: () => _setAndCommit(widget.defaultWidth),
            child: const SizedBox(
              width: LibraryNavigationResizeHandle.hitTargetWidth,
            ),
          ),
        ),
      ),
    );
  }

  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }
    if (event.logicalKey == LogicalKeyboardKey.arrowLeft) {
      _setAndCommit(widget.width - _keyboardStep);
      return KeyEventResult.handled;
    }
    if (event.logicalKey == LogicalKeyboardKey.arrowRight) {
      _setAndCommit(widget.width + _keyboardStep);
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  void _setAndCommit(double width) {
    final nextWidth = _bounded(width);
    widget.onWidthChangeStart();
    widget.onWidthChanged(nextWidth);
    widget.onWidthChangeEnd(nextWidth);
  }

  void _clearDragState() {
    _dragStartWidth = null;
    _dragStartGlobalX = null;
    _pendingWidth = null;
  }

  double _bounded(double width) {
    return width.clamp(widget.minimumWidth, widget.maximumWidth).toDouble();
  }
}
