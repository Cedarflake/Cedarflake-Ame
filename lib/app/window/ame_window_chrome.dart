import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";
import "package:window_manager/window_manager.dart";

import "ame_window_actions.dart";

class AmeWindowDragRegion extends StatelessWidget {
  const AmeWindowDragRegion({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DragToMoveArea(child: child);
  }
}

class AmeWindowCaptionControls extends ConsumerWidget {
  const AmeWindowCaptionControls({this.height = 40, super.key});

  final double height;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final actions = ref.watch(ameWindowActionsProvider);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        IconButton(
          key: const Key("window-minimize"),
          tooltip: "最小化",
          onPressed: () => unawaited(actions.minimize()),
          style: _captionButtonStyle(height),
          icon: const _CaptionSymbol(Symbols.horizontal_rule_rounded, size: 22),
        ),
        ValueListenableBuilder<bool>(
          valueListenable: actions.isMaximized,
          builder: (context, isMaximized, child) => IconButton(
            key: const Key("window-maximize"),
            tooltip: isMaximized ? "还原" : "最大化",
            onPressed: () => unawaited(actions.toggleMaximize()),
            style: _captionButtonStyle(height),
            icon: _CaptionSymbol(
              isMaximized
                  ? Symbols.filter_none_rounded
                  : Symbols.crop_square_rounded,
              size: 18,
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(right: 8),
          child: IconButton(
            key: const Key("window-close"),
            tooltip: "关闭",
            onPressed: () => unawaited(actions.close()),
            style: _captionButtonStyle(height),
            icon: const _CaptionSymbol(Symbols.close_rounded, size: 24),
          ),
        ),
      ],
    );
  }
}

class _CaptionSymbol extends StatelessWidget {
  const _CaptionSymbol(this.symbol, {required this.size});

  final IconData symbol;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Icon(
      symbol,
      size: size,
      fill: 0,
      weight: 400,
      grade: 0,
      opticalSize: 20,
    );
  }
}

ButtonStyle _captionButtonStyle(double height) {
  final diameter = height < 48 ? height : 48.0;
  return ButtonStyle(
    fixedSize: WidgetStatePropertyAll(Size.square(diameter)),
    minimumSize: WidgetStatePropertyAll(Size.square(diameter)),
    padding: const WidgetStatePropertyAll(EdgeInsets.zero),
    shape: const WidgetStatePropertyAll(CircleBorder()),
  );
}
