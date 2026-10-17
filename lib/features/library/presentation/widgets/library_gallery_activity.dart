import "package:flutter/foundation.dart";
import "package:flutter/widgets.dart";

import "../../application/library_controller.dart";

/// Retains gallery state while releasing preview work for a hidden workspace.
class LibraryGalleryActivity extends StatefulWidget {
  const LibraryGalleryActivity({
    required this.controller,
    required this.child,
    super.key,
  });

  final LibraryController controller;
  final Widget child;

  @override
  State<LibraryGalleryActivity> createState() => _LibraryGalleryActivityState();
}

class _LibraryGalleryActivityState extends State<LibraryGalleryActivity> {
  ValueListenable<TickerModeData>? _activity;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final activity = TickerMode.getValuesNotifier(context);
    if (identical(activity, _activity)) {
      return;
    }
    _activity?.removeListener(_onActivityChanged);
    _activity = activity..addListener(_onActivityChanged);
    _onActivityChanged();
  }

  void _onActivityChanged() {
    if (_activity?.value.enabled != false) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _activity?.value.enabled == false) {
        widget.controller.updateGalleryPreviewDemand();
      }
    });
  }

  @override
  void dispose() {
    _activity?.removeListener(_onActivityChanged);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
