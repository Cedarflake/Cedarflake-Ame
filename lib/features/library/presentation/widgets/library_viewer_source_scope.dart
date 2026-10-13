import "package:flutter/widgets.dart";

import "../../application/library_source_read_scheduler.dart";
import "library_source_image.dart";

class LibraryViewerSourceScope extends InheritedWidget {
  const LibraryViewerSourceScope({
    required this.scheduler,
    required this.bufferLoader,
    required super.child,
    super.key,
  });

  final LibrarySourceReadScheduler scheduler;
  final LibrarySourceBufferLoader bufferLoader;

  static LibraryViewerSourceScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryViewerSourceScope>();

  @override
  bool updateShouldNotify(LibraryViewerSourceScope oldWidget) =>
      !identical(scheduler, oldWidget.scheduler) ||
      bufferLoader != oldWidget.bufferLoader;
}
