import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../adapters/windows_library_platform_actions.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_asset_information_sheet.dart";
import "library_image_viewer.dart";
import "library_viewer_transition.dart";

class LibraryDuplicateReviewViewer extends ConsumerStatefulWidget {
  const LibraryDuplicateReviewViewer({
    required this.initialAsset,
    required this.initialIndex,
    required this.totalCount,
    required this.loadAsset,
    required this.onBack,
    super.key,
  });
  final LibraryAsset initialAsset;
  final int initialIndex;
  final int totalCount;
  final Future<LibraryAsset?> Function(int index) loadAsset;
  final VoidCallback onBack;

  @override
  ConsumerState<LibraryDuplicateReviewViewer> createState() =>
      _LibraryDuplicateReviewViewerState();
}

class _LibraryDuplicateReviewViewerState
    extends ConsumerState<LibraryDuplicateReviewViewer> {
  late final _library = ref.read(libraryControllerProvider.notifier);
  late int _index = widget.initialIndex;
  late LibraryAsset _asset = widget.initialAsset;
  bool _closing = false;
  bool _navigating = false;

  void _back() {
    setState(() => _closing = true);
    _library.updateViewerPreviewDemand(null);
    widget.onBack();
  }

  Future<void> _navigate(int index) async {
    if (_navigating || _closing) return;
    setState(() => _navigating = true);
    final asset = await widget.loadAsset(index);
    if (!mounted || _closing) return;
    setState(() {
      _navigating = false;
      if (asset != null) {
        _asset = asset;
        _index = index;
      }
    });
    if (asset == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text(LibraryDuplicateStrings.loadFailed)),
      );
    }
    _library.updateViewerPreviewDemand(_asset);
  }

  @override
  void initState() {
    super.initState();
    _library.updateViewerPreviewDemand(_asset);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _library.updateViewerPreviewDemand(
      !_closing && LibraryViewerActivity.isActiveOf(context) ? _asset : null,
    );
  }

  @override
  void dispose() {
    _library.updateViewerPreviewDemand(null);
    super.dispose();
  }

  Future<void> _reveal() async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealFile(_asset.displayPath);
    } on Object {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text(LibraryDuplicateStrings.revealFailed)),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final preferences = ref.watch(amePreferencesControllerProvider);
    return StreamBuilder<void>(
      stream: _library.watchPreview(_asset.locationId),
      builder: (context, snapshot) {
        final asset = _library.resolvePreview(_asset);
        return LibraryViewerActivity(
          isActive: !_closing,
          child: LibraryImageViewer(
            asset: asset,
            onBack: _back,
            onPrevious: !_navigating && _index > 0
                ? () => unawaited(_navigate(_index - 1))
                : null,
            onNext: !_navigating && _index + 1 < widget.totalCount
                ? () => unawaited(_navigate(_index + 1))
                : null,
            openBehavior: preferences.viewerOpenBehavior,
            wheelBehavior: preferences.viewerWheelBehavior,
            onInformation: () =>
                unawaited(showLibraryAssetInformation(context, asset)),
            onCopyPath: () => unawaited(
              Clipboard.setData(ClipboardData(text: asset.displayPath)),
            ),
            onRevealFile: () => unawaited(_reveal()),
          ),
        );
      },
    );
  }
}
