import "package:material_ui/material_ui.dart";

class AmeBrandMark extends StatelessWidget {
  const AmeBrandMark({this.size = 28, super.key});

  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return SizedBox.square(
      dimension: size,
      child: Image.asset(
        "assets/branding/ame_light.png",
        color: colors.primary,
        colorBlendMode: BlendMode.srcIn,
        fit: BoxFit.contain,
        filterQuality: FilterQuality.medium,
        excludeFromSemantics: true,
      ),
    );
  }
}
