import "package:flutter/material.dart";

class AmeBrandMark extends StatelessWidget {
  const AmeBrandMark({this.size = 28, super.key});

  final double size;

  @override
  Widget build(BuildContext context) {
    return SizedBox.square(
      dimension: size,
      child: Image.asset(
        "assets/branding/ame_outline.png",
        color: Theme.of(context).colorScheme.primary,
        colorBlendMode: BlendMode.srcIn,
        filterQuality: FilterQuality.medium,
        excludeFromSemantics: true,
      ),
    );
  }
}
