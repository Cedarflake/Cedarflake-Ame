import "package:flutter/material.dart";

class AmeBrandMark extends StatelessWidget {
  const AmeBrandMark({this.size = 28, super.key});

  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return SizedBox.square(
      dimension: size,
      child: Stack(
        fit: StackFit.expand,
        children: [
          Image.asset(
            "assets/branding/ame_light.png",
            color: colors.primaryContainer,
            colorBlendMode: BlendMode.srcIn,
            filterQuality: FilterQuality.medium,
            excludeFromSemantics: true,
          ),
          Image.asset(
            "assets/branding/ame_outline.png",
            color: colors.primary,
            colorBlendMode: BlendMode.srcIn,
            filterQuality: FilterQuality.medium,
            excludeFromSemantics: true,
          ),
        ],
      ),
    );
  }
}
