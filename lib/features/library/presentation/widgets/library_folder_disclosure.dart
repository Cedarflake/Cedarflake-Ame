import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";

class LibraryFolderDisclosure extends StatefulWidget {
  const LibraryFolderDisclosure({required this.child, super.key});

  final Widget? child;

  @override
  State<LibraryFolderDisclosure> createState() =>
      _LibraryFolderDisclosureState();
}

class _LibraryFolderDisclosureState extends State<LibraryFolderDisclosure>
    with TickerProviderStateMixin {
  late Widget? _retainedChild = widget.child;
  late final AnimationController _size;
  late final AnimationController _opacity;

  @override
  void initState() {
    super.initState();
    final initial = widget.child == null ? 0.0 : 1.0;
    _size = AnimationController(vsync: this, value: initial)
      ..addStatusListener(_retireClosedChild);
    _opacity = AnimationController(vsync: this, value: initial)
      ..addStatusListener(_retireClosedChild);
  }

  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _finish();
  }

  @override
  void didUpdateWidget(covariant LibraryFolderDisclosure oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.child != null) _retainedChild = widget.child;
    if (_skipMotion) {
      _finish();
    } else if ((widget.child == null) != (oldWidget.child == null)) {
      final opening = widget.child != null;
      AmeMotion.fastSpatial.animate(_size, forward: opening);
      AmeMotion.fastEffects.animate(_opacity, forward: opening);
    }
  }

  void _finish() {
    final target = widget.child == null ? 0.0 : 1.0;
    _size.value = target;
    _opacity.value = target;
    _retainedChild = widget.child;
  }

  void _retireClosedChild(AnimationStatus _) {
    if (widget.child == null &&
        _size.isDismissed &&
        _opacity.isDismissed &&
        _retainedChild != null) {
      setState(() => _retainedChild = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final child = _retainedChild;
    if (child == null) return const SizedBox.shrink();
    final closing = widget.child == null;
    return SizeTransition(
      sizeFactor: _size,
      alignment: Alignment.topCenter,
      child: FadeTransition(
        opacity: _opacity,
        child: IgnorePointer(
          ignoring: closing,
          child: ExcludeFocus(
            excluding: closing,
            child: ExcludeSemantics(
              excluding: closing,
              child: TickerMode(
                enabled: !closing,
                child: Material(type: MaterialType.transparency, child: child),
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _size.dispose();
    _opacity.dispose();
    super.dispose();
  }
}
