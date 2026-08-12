import "package:flutter/material.dart";

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

class AmeMenuAnchor extends StatelessWidget {
  const AmeMenuAnchor({
    required this.menuChildren,
    this.controller,
    this.childFocusNode,
    this.style,
    this.alignmentOffset,
    this.reservedPadding = const EdgeInsets.all(AmeMenuMetrics.viewportPadding),
    this.builder,
    this.child,
    super.key,
  });

  final MenuController? controller;
  final FocusNode? childFocusNode;
  final MenuStyle? style;
  final Offset? alignmentOffset;
  final EdgeInsetsGeometry reservedPadding;
  final List<Widget> menuChildren;
  final MenuAnchorChildBuilder? builder;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final anchorBuilder = builder;
    final anchorChild = child;
    final isolatedBuilder = anchorBuilder == null
        ? null
        : (BuildContext context, MenuController controller, Widget? child) =>
              _AmeMenuTraversalBoundary(
                child: anchorBuilder(context, controller, child),
              );
    return _AmeMenuTraversalBoundary(
      child: MenuAnchor(
        controller: controller,
        childFocusNode: childFocusNode,
        style: style,
        alignmentOffset: alignmentOffset ?? Offset.zero,
        reservedPadding: reservedPadding,
        animated: true,
        menuChildren: menuChildren,
        builder: isolatedBuilder,
        child: anchorChild == null
            ? null
            : _AmeMenuTraversalBoundary(child: anchorChild),
      ),
    );
  }
}

// Flutter 3.44 can merge nested OverlayPortal traversal parents into one
// semantics node. Keep the menu portal and its tooltip anchor separate until
// flutter/flutter#190344 and flutter/flutter#190431 reach the pinned SDK.
class _AmeMenuTraversalBoundary extends StatelessWidget {
  const _AmeMenuTraversalBoundary({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Semantics(container: true, explicitChildNodes: true, child: child);
  }
}

MenuStyle ameFixedWidthMenuStyle(double width) {
  return MenuStyle(
    minimumSize: WidgetStatePropertyAll(Size(width, 0)),
    maximumSize: WidgetStatePropertyAll(Size(width, double.infinity)),
  );
}

Offset ameMenuBelowEndAlignment({
  required double menuWidth,
  double anchorWidth = 48,
  double endOffset = 0,
  double verticalGap = 4,
}) {
  return Offset(anchorWidth - menuWidth + endOffset, verticalGap);
}

Widget ameFixedWidthMenuItem({required double width, required Widget child}) {
  return SizedBox(width: width, child: child);
}

void toggleAmeMenu(MenuController controller) {
  if (controller.isOpen) {
    controller.close();
  } else {
    controller.open();
  }
}

class AmeMenuItemContent extends StatelessWidget {
  const AmeMenuItemContent({
    required this.icon,
    required this.label,
    this.shortcut,
    super.key,
  });

  final IconData icon;
  final String label;
  final String? shortcut;

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
            child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
        ),
        if (shortcut case final shortcut?) ...[
          const SizedBox(width: AmeMenuMetrics.shortcutGap),
          Text(shortcut, maxLines: 1, softWrap: false),
        ],
      ],
    );
  }
}

double amePopupMenuContentWidth({
  required BuildContext context,
  required Iterable<String> labels,
  Iterable<String> shortcuts = const [],
  bool hasLeadingIcon = true,
}) {
  final popupMenuTheme = PopupMenuTheme.of(context);
  final textStyle =
      popupMenuTheme.labelTextStyle?.resolve(const <WidgetState>{}) ??
      Theme.of(context).textTheme.labelLarge ??
      const TextStyle();
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
  final decorationWidth = hasLeadingIcon
      ? (AmeMenuMetrics.horizontalPadding * 2) +
            AmeMenuMetrics.iconSize +
            AmeMenuMetrics.iconLabelGap
      : AmeMenuMetrics.horizontalPadding * 2;
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
}) {
  return showMenu<T>(
    context: context,
    useRootNavigator: true,
    position: position,
    constraints: BoxConstraints.tightFor(
      width: amePopupMenuContentWidth(
        context: context,
        labels: labels,
        shortcuts: shortcuts,
      ),
    ),
    items: items,
  );
}
