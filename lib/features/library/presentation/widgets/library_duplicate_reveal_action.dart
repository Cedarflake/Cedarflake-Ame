import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../adapters/windows_library_platform_actions.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";

/// Shares availability, in-flight ownership and error feedback across reveal controls.
class LibraryDuplicateRevealAction extends ConsumerStatefulWidget {
  const LibraryDuplicateRevealAction({
    required this.location,
    required this.builder,
    super.key,
  });

  final LibraryDuplicateLocation location;
  final Widget Function(BuildContext context, VoidCallback? reveal) builder;

  @override
  ConsumerState<LibraryDuplicateRevealAction> createState() =>
      _LibraryDuplicateRevealActionState();
}

class _LibraryDuplicateRevealActionState
    extends ConsumerState<LibraryDuplicateRevealAction> {
  bool _opening = false;

  Future<void> _reveal() async {
    if (_opening) return;
    setState(() => _opening = true);
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealFile(widget.location.displayPath);
    } on Object {
      if (mounted) {
        showAmeTransientFeedback(
          context,
          message: LibraryDuplicateStrings.revealFailed,
          severity: AmeNotificationSeverity.error,
        );
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final available = switch (widget.location.availability) {
      LibraryDuplicateAvailability.local ||
      LibraryDuplicateAvailability.cloudOnly => true,
      LibraryDuplicateAvailability.offline ||
      LibraryDuplicateAvailability.unavailable => false,
    };
    return widget.builder(context, available && !_opening ? _reveal : null);
  }
}
