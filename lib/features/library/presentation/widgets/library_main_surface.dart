import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_workbench_surfaces.dart";

class LibraryMainSurface extends StatelessWidget {
  const LibraryMainSurface({required this.child, super.key});

  static const borderRadius = BorderRadius.all(Radius.circular(24));

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: AmeWorkbenchSurfaces.of(context).content,
        borderRadius: borderRadius,
      ),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: Material(
          key: const Key("library-main-surface"),
          type: MaterialType.transparency,
          child: child,
        ),
      ),
    );
  }
}
