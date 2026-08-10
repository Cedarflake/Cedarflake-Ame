import "package:flutter/material.dart";

abstract final class AmeMenuMetrics {
  static const double minimumWidth = 112;
  static const double maximumWidth = 280;
  static const double itemHeight = 48;
  static const double iconSize = 24;
  static const double iconLabelGap = 12;
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
    this.builder,
    this.child,
    super.key,
  });

  final MenuController? controller;
  final FocusNode? childFocusNode;
  final List<Widget> menuChildren;
  final MenuAnchorChildBuilder? builder;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    return MenuAnchor(
      controller: controller,
      childFocusNode: childFocusNode,
      animated: true,
      menuChildren: menuChildren,
      builder: builder,
      child: child,
    );
  }
}

class AmeMenuItemContent extends StatelessWidget {
  const AmeMenuItemContent({
    required this.icon,
    required this.label,
    super.key,
  });

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: AmeMenuMetrics.iconSize),
        const SizedBox(width: AmeMenuMetrics.iconLabelGap),
        ConstrainedBox(
          constraints: const BoxConstraints(
            maxWidth: AmeMenuMetrics.maximumLabelWidth,
          ),
          child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        ),
      ],
    );
  }
}

double amePopupMenuContentWidth({
  required BuildContext context,
  required Iterable<String> labels,
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
  var maximumTextWidth = 0.0;
  for (final label in labels) {
    textPainter.text = TextSpan(text: label, style: textStyle);
    textPainter.layout();
    if (textPainter.width > maximumTextWidth) {
      maximumTextWidth = textPainter.width;
    }
  }
  textPainter.dispose();
  final decorationWidth = hasLeadingIcon
      ? (AmeMenuMetrics.horizontalPadding * 2) +
            AmeMenuMetrics.iconSize +
            AmeMenuMetrics.iconLabelGap
      : AmeMenuMetrics.horizontalPadding * 2;
  return (maximumTextWidth + decorationWidth)
      .clamp(AmeMenuMetrics.minimumWidth, AmeMenuMetrics.maximumWidth)
      .ceilToDouble();
}

Future<T?> showAmePopupMenu<T>({
  required BuildContext context,
  required RelativeRect position,
  required Iterable<String> labels,
  required List<PopupMenuEntry<T>> items,
}) {
  return showMenu<T>(
    context: context,
    useRootNavigator: true,
    position: position,
    constraints: BoxConstraints.tightFor(
      width: amePopupMenuContentWidth(context: context, labels: labels),
    ),
    items: items,
  );
}
