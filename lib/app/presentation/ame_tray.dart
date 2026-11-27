import "package:material_ui/material_ui.dart";

Future<T?> showAmeTray<T>({
  required BuildContext context,
  required WidgetBuilder builder,
}) => showModalBottomSheet<T>(
  context: context,
  showDragHandle: true,
  isScrollControlled: true,
  useSafeArea: true,
  enableDrag: false,
  builder: builder,
);

class AmeTray extends StatelessWidget {
  const AmeTray({required this.child, super.key});
  final Widget child;

  @override
  Widget build(BuildContext context) => SafeArea(
    child: Padding(
      padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(context).height * .8,
        ),
        child: child,
      ),
    ),
  );
}
