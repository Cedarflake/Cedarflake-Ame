import "package:flutter/widgets.dart";

class AmeApplicationMount {
  bool _isMounted = false;

  void mount(Widget application) {
    runApp(application);
    _isMounted = true;
  }

  Future<void> unmount() async {
    if (!_isMounted) return;
    _isMounted = false;
    // runApp schedules a warm-up frame even after the native window is hidden.
    runApp(const SizedBox.shrink());
    await WidgetsBinding.instance.endOfFrame;
  }
}
