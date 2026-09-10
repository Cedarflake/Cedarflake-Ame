import "dart:async";

import "package:flutter/material.dart";

import "ame_popup_menu_position.dart";
import "ame_typography.dart";

abstract final class AmeMenuMetrics {
  static const double minimumWidth = 112;
  static const double maximumWidth = 280;
  static const double itemHeight = 48;
  static const double iconSize = 24;
  static const double iconLabelGap = 12;
  static const double selectionIndicatorSlotWidth = 8;
  static const double selectionIndicatorSize = 6;
  static const double shortcutGap = 24;
  static const double horizontalPadding = 12;
  static const double verticalPadding = 8;
  static const double dividerHeight = 1;
  static const double elevation = 3;
  static const double borderRadius = 4;
  static const double viewportPadding = 12;
  static const double maximumLabelWidth =
      maximumWidth - (horizontalPadding * 2) - iconSize - iconLabelGap;

  static const EdgeInsets menuPadding = EdgeInsets.symmetric(
    vertical: verticalPadding,
  );
  static const EdgeInsets itemPadding = EdgeInsets.symmetric(
    horizontal: horizontalPadding,
  );
  static const RoundedRectangleBorder shape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(borderRadius)),
  );
}

abstract final class AmeMenuTransition {
  static const Duration duration = Duration(milliseconds: 200);
  static const Curve curve = Curves.easeOutCubic;
  static const Curve reverseCurve = Curves.easeInCubic;
  static const AnimationStyle popupAnimationStyle = AnimationStyle(
    curve: curve,
    duration: duration,
    reverseCurve: reverseCurve,
    reverseDuration: duration,
  );
}

MenuThemeData buildAmeMenuTheme(ColorScheme colorScheme) {
  return MenuThemeData(
    style: MenuStyle(
      backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainer),
      shadowColor: WidgetStatePropertyAll(colorScheme.shadow),
      surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
      elevation: const WidgetStatePropertyAll(AmeMenuMetrics.elevation),
      padding: const WidgetStatePropertyAll(AmeMenuMetrics.menuPadding),
      minimumSize: const WidgetStatePropertyAll(
        Size(AmeMenuMetrics.minimumWidth, 0),
      ),
      maximumSize: const WidgetStatePropertyAll(
        Size(AmeMenuMetrics.maximumWidth, double.infinity),
      ),
      shape: const WidgetStatePropertyAll(AmeMenuMetrics.shape),
    ),
  );
}

MenuButtonThemeData buildAmeMenuButtonTheme() {
  return const MenuButtonThemeData(
    style: ButtonStyle(
      padding: WidgetStatePropertyAll(AmeMenuMetrics.itemPadding),
      minimumSize: WidgetStatePropertyAll(
        Size(AmeMenuMetrics.minimumWidth, AmeMenuMetrics.itemHeight),
      ),
      maximumSize: WidgetStatePropertyAll(
        Size(AmeMenuMetrics.maximumWidth, double.infinity),
      ),
      iconSize: WidgetStatePropertyAll(AmeMenuMetrics.iconSize),
      alignment: AlignmentDirectional.centerStart,
    ),
  );
}

PopupMenuThemeData buildAmePopupMenuTheme(ColorScheme colorScheme) {
  return PopupMenuThemeData(
    color: colorScheme.surfaceContainer,
    shape: AmeMenuMetrics.shape,
    menuPadding: AmeMenuMetrics.menuPadding,
    elevation: AmeMenuMetrics.elevation,
    shadowColor: colorScheme.shadow,
    surfaceTintColor: Colors.transparent,
  );
}

typedef AmePopupMenuButtonBuilder =
    Widget Function(BuildContext context, VoidCallback openMenu);

class AmePopupMenuButton<T> extends StatefulWidget {
  const AmePopupMenuButton({
    required this.labels,
    required this.items,
    required this.onSelected,
    required this.builder,
    this.shortcuts = const [],
    this.menuWidth,
    this.maximumHeight,
    this.leadingIconWidth = 0,
    this.verticalGap = 4,
    this.viewportRightMargin,
    this.onOpened,
    this.onOpenChanged,
    this.initialValue,
    super.key,
  });

  final Iterable<String> labels;
  final Iterable<String> shortcuts;
  final List<PopupMenuEntry<T>> items;
  final ValueChanged<T> onSelected;
  final AmePopupMenuButtonBuilder builder;
  final double? menuWidth;
  final double? maximumHeight;
  final double leadingIconWidth;
  final double verticalGap;
  final double? viewportRightMargin;
  final VoidCallback? onOpened;
  final ValueChanged<bool>? onOpenChanged;
  final T? initialValue;

  @override
  State<AmePopupMenuButton<T>> createState() => _AmePopupMenuButtonState<T>();
}

class _AmePopupMenuButtonState<T> extends State<AmePopupMenuButton<T>> {
  final GlobalKey _anchorKey = GlobalKey(debugLabel: "Ame popup menu anchor");
  bool _isRouteOpen = false;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      key: _anchorKey,
      child: widget.builder(context, _requestOpen),
    );
  }

  void _requestOpen() {
    if (!_isRouteOpen) {
      unawaited(_openMenu());
    }
  }

  Future<void> _openMenu() async {
    final anchorContext = _anchorKey.currentContext;
    if (anchorContext == null) {
      return;
    }
    final position = amePopupMenuBelowAnchor(
      context: context,
      anchorContext: anchorContext,
      verticalGap: widget.verticalGap,
      viewportRightMargin: widget.viewportRightMargin,
    );
    if (position == null) {
      return;
    }
    _isRouteOpen = true;
    try {
      widget.onOpenChanged?.call(true);
      widget.onOpened?.call();
      final selected = await showAmePopupMenu<T>(
        context: context,
        position: position,
        labels: widget.labels,
        shortcuts: widget.shortcuts,
        items: widget.items,
        menuWidth: widget.menuWidth,
        maximumHeight: widget.maximumHeight,
        leadingIconWidth: widget.leadingIconWidth,
        initialValue: widget.initialValue,
      );
      if (mounted && selected != null) {
        widget.onSelected(selected);
      }
    } finally {
      _isRouteOpen = false;
      if (mounted) {
        widget.onOpenChanged?.call(false);
      }
    }
  }
}

class AmeMenuItemContent extends StatelessWidget {
  const AmeMenuItemContent({
    required this.icon,
    required this.label,
    this.shortcut,
    this.isSelected = false,
    super.key,
  });

  final IconData icon;
  final String label;
  final String? shortcut;
  final bool isSelected;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: AmeMenuMetrics.iconSize),
        const SizedBox(width: AmeMenuMetrics.iconLabelGap),
        Flexible(
          child: ConstrainedBox(
            constraints: const BoxConstraints(
              maxWidth: AmeMenuMetrics.maximumLabelWidth,
            ),
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                fontWeight: isSelected
                    ? ameFontWeightSemibold
                    : ameFontWeightMedium,
              ),
            ),
          ),
        ),
        if (shortcut case final shortcut?) ...[
          const SizedBox(width: AmeMenuMetrics.shortcutGap),
          Text(
            shortcut,
            maxLines: 1,
            softWrap: false,
            style: Theme.of(context).textTheme.labelMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
              fontWeight: ameFontWeightRegular,
            ),
          ),
        ],
      ],
    );
  }
}

double amePopupMenuContentWidth({
  required BuildContext context,
  required Iterable<String> labels,
  Iterable<String> shortcuts = const [],
  bool hasContentIcon = true,
  double leadingIconWidth = 0,
}) {
  final popupMenuTheme = PopupMenuTheme.of(context);
  final textStyle =
      (popupMenuTheme.labelTextStyle?.resolve(const <WidgetState>{}) ??
              Theme.of(context).textTheme.labelLarge ??
              const TextStyle())
          .copyWith(fontWeight: ameFontWeightSemibold);
  final textPainter = TextPainter(
    textDirection: Directionality.of(context),
    textScaler: MediaQuery.textScalerOf(context),
    maxLines: 1,
  );
  final labelList = labels.toList(growable: false);
  final shortcutList = shortcuts.toList(growable: false);
  var maximumContentWidth = 0.0;
  for (var index = 0; index < labelList.length; index += 1) {
    final label = labelList[index];
    textPainter.text = TextSpan(text: label, style: textStyle);
    textPainter.layout();
    var contentWidth = textPainter.width;
    if (index < shortcutList.length) {
      textPainter.text = TextSpan(text: shortcutList[index], style: textStyle);
      textPainter.layout();
      contentWidth += AmeMenuMetrics.shortcutGap + textPainter.width;
    }
    if (contentWidth > maximumContentWidth) {
      maximumContentWidth = contentWidth;
    }
  }
  textPainter.dispose();
  final contentIconWidth = hasContentIcon
      ? AmeMenuMetrics.iconSize + AmeMenuMetrics.iconLabelGap
      : 0;
  final leadingDecorationWidth = leadingIconWidth > 0
      ? leadingIconWidth + AmeMenuMetrics.iconLabelGap
      : 0;
  final decorationWidth =
      (AmeMenuMetrics.horizontalPadding * 2) +
      contentIconWidth +
      leadingDecorationWidth;
  return (maximumContentWidth + decorationWidth)
      .clamp(AmeMenuMetrics.minimumWidth, AmeMenuMetrics.maximumWidth)
      .ceilToDouble();
}

Future<T?> showAmePopupMenu<T>({
  required BuildContext context,
  required RelativeRect position,
  required Iterable<String> labels,
  Iterable<String> shortcuts = const [],
  required List<PopupMenuEntry<T>> items,
  double? menuWidth,
  double? maximumHeight,
  double leadingIconWidth = 0,
  T? initialValue,
}) {
  final resolvedWidth =
      menuWidth ??
      amePopupMenuContentWidth(
        context: context,
        labels: labels,
        shortcuts: shortcuts,
        leadingIconWidth: leadingIconWidth,
      );
  return showMenu<T>(
    context: context,
    useRootNavigator: true,
    requestFocus: true,
    position: position,
    initialValue: initialValue,
    popUpAnimationStyle: AmeMenuTransition.popupAnimationStyle,
    constraints: BoxConstraints(
      minWidth: resolvedWidth,
      maxWidth: resolvedWidth,
      maxHeight: maximumHeight ?? double.infinity,
    ),
    items: items,
  );
}
