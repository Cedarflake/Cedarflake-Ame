import "package:flutter/widgets.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

typedef LibrarySettingsDestination =
    Widget Function(
      BuildContext context, {
      required bool hasLibraryRoots,
      required Set<String> libraryRootIds,
    });

typedef LibraryRecordFeedback = Widget? Function(BuildContext, WidgetRef);

final librarySettingsDestinationProvider = Provider<LibrarySettingsDestination>(
  (ref) => throw StateError("Library settings destination binding is required"),
);

final libraryRecordFeedbackProvider = Provider<LibraryRecordFeedback>(
  (ref) => throw StateError("Library record feedback binding is required"),
);
