import "dart:io";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../application/library_controller.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "../../storage/presentation/storage_settings_dialog.dart";

class LibraryScreen extends ConsumerWidget {
  const LibraryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryControllerProvider);
    final controller = ref.read(libraryControllerProvider.notifier);

    return Scaffold(
      appBar: AppBar(
        automaticallyImplyLeading: false,
        titleSpacing: 20,
        title: const Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.photo_library_outlined),
            SizedBox(width: 12),
            Text("Ame"),
          ],
        ),
        actions: [
          _TaskActivityButton(
            state: state,
            onPause: controller.pauseScan,
            onCancel: controller.cancelScan,
            onResume: controller.resumePausedScan,
            onRetry: controller.retry,
          ),
          TextButton.icon(
            key: const Key("library-import-button"),
            onPressed: state.isBusy ? null : controller.chooseDirectoryAndScan,
            icon: const Icon(Icons.add_photo_alternate_outlined),
            label: const Text("Import"),
          ),
          StorageSettingsButton(hasLibraryRoots: state.roots.isNotEmpty),
          const SizedBox(width: 16),
        ],
      ),
      body: Row(
        children: [
          _LibraryNavigation(state: state),
          const VerticalDivider(width: 1),
          Expanded(
            child: _LibraryCanvas(
              state: state,
              onImport: controller.chooseDirectoryAndScan,
              onRetry: controller.retry,
              onLoadMore: controller.loadNextPage,
            ),
          ),
        ],
      ),
    );
  }
}

enum _TaskAction { pause, cancel, resume, retry }

class _TaskActivityButton extends StatelessWidget {
  const _TaskActivityButton({
    required this.state,
    required this.onPause,
    required this.onCancel,
    required this.onResume,
    required this.onRetry,
  });

  final LibraryState state;
  final VoidCallback onPause;
  final VoidCallback onCancel;
  final Future<void> Function() onResume;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<_TaskAction>(
      key: const Key("library-task-activity-button"),
      tooltip: "Task activity",
      icon: Icon(state.isProcessing ? Icons.sync : Icons.task_alt_outlined),
      onSelected: (action) {
        switch (action) {
          case _TaskAction.pause:
            onPause();
            break;
          case _TaskAction.cancel:
            onCancel();
            break;
          case _TaskAction.resume:
            onResume();
            break;
          case _TaskAction.retry:
            onRetry();
            break;
        }
      },
      itemBuilder: (context) => [
        PopupMenuItem<_TaskAction>(
          enabled: false,
          child: SizedBox(
            width: 280,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _taskTitle(state),
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 4),
                Text(
                  _taskDetails(state),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
        ),
        if (state.status == LibraryStatus.scanning) ...[
          const PopupMenuDivider(),
          const PopupMenuItem(
            key: Key("library-pause-button"),
            value: _TaskAction.pause,
            child: ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.pause_outlined),
              title: Text("Pause import"),
            ),
          ),
          const PopupMenuItem(
            key: Key("library-cancel-button"),
            value: _TaskAction.cancel,
            child: ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.stop_circle_outlined),
              title: Text("Cancel import"),
            ),
          ),
        ] else if (state.status == LibraryStatus.paused) ...[
          const PopupMenuDivider(),
          const PopupMenuItem(
            key: Key("library-resume-button"),
            value: _TaskAction.resume,
            child: ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.play_arrow_outlined),
              title: Text("Resume import"),
            ),
          ),
        ] else if (state.status == LibraryStatus.failed ||
            state.status == LibraryStatus.cancelled ||
            state.status == LibraryStatus.stale) ...[
          const PopupMenuDivider(),
          const PopupMenuItem(
            key: Key("library-retry-button"),
            value: _TaskAction.retry,
            child: ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.refresh),
              title: Text("Retry import"),
            ),
          ),
        ],
      ],
    );
  }

  static String _taskTitle(LibraryState state) {
    return switch (state.status) {
      LibraryStatus.empty || LibraryStatus.completed => "No active tasks",
      LibraryStatus.choosingDirectory => "Choosing a source",
      LibraryStatus.scanning =>
        state.isResumingScan ? "Resuming import" : "Importing photos",
      LibraryStatus.pausing => "Pausing import",
      LibraryStatus.cancelling => "Cancelling import",
      LibraryStatus.refreshing => "Updating library",
      LibraryStatus.cancelled => "Import cancelled",
      LibraryStatus.paused => "Import paused",
      LibraryStatus.stale => "Source changed during import",
      LibraryStatus.failed => "Import failed",
    };
  }

  static String _taskDetails(LibraryState state) {
    if (state.isScanning || state.status == LibraryStatus.paused) {
      return "${state.visitedEntries} entries checked, "
          "${state.stagedAssetCount} images found";
    }
    if (state.status == LibraryStatus.failed && state.errorMessage != null) {
      return state.errorMessage!;
    }
    return "Imports and background work appear here.";
  }
}

class _LibraryNavigation extends StatelessWidget {
  const _LibraryNavigation({required this.state});

  final LibraryState state;

  @override
  Widget build(BuildContext context) {
    final rootPath = state.rootPath;
    final hasTransientRoot =
        rootPath != null && !state.roots.any((root) => root.path == rootPath);

    return SizedBox(
      width: 252,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 16, 12, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ListTile(
              selected: true,
              leading: const Icon(Icons.photo_library_outlined),
              title: const Text("Library"),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(28),
              ),
            ),
            const SizedBox(height: 24),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(
                "SOURCES",
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: ListView(
                padding: EdgeInsets.zero,
                children: [
                  if (state.roots.isEmpty && rootPath == null)
                    const ListTile(
                      leading: Icon(Icons.folder_off_outlined),
                      title: Text("No folder imported"),
                    )
                  else ...[
                    for (final root in state.roots)
                      _SourceTile(
                        path: root.path,
                        assetCount: root.assetCount,
                        availability: root.availability,
                        availabilityMessage: root.availabilityMessage,
                      ),
                    if (hasTransientRoot)
                      _SourceTile(path: rootPath, isPending: true),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SourceTile extends StatelessWidget {
  const _SourceTile({
    required this.path,
    this.assetCount,
    this.availability = LibraryRootAvailability.unknown,
    this.availabilityMessage,
    this.isPending = false,
  });

  final String path;
  final int? assetCount;
  final LibraryRootAvailability availability;
  final String? availabilityMessage;
  final bool isPending;

  @override
  Widget build(BuildContext context) {
    final (icon, statusText) = switch (availability) {
      LibraryRootAvailability.available => (Icons.folder_outlined, null),
      LibraryRootAvailability.missing => (
        Icons.folder_off_outlined,
        "Source missing",
      ),
      LibraryRootAvailability.inaccessible => (
        Icons.lock_outline,
        "Source inaccessible",
      ),
      LibraryRootAvailability.offline => (
        Icons.cloud_off_outlined,
        "Source offline",
      ),
      LibraryRootAvailability.unknown => (Icons.folder_outlined, null),
    };
    final subtitle = isPending
        ? "Importing"
        : statusText == null
        ? "${assetCount ?? 0} images"
        : "${assetCount ?? 0} images - $statusText";
    return Tooltip(
      message: availabilityMessage == null
          ? path
          : "$path\n$availabilityMessage",
      child: ListTile(
        leading: Icon(isPending ? Icons.pending_outlined : icon),
        title: Text(
          _folderName(path),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis),
      ),
    );
  }

  static String _folderName(String path) {
    final segments = path
        .replaceAll("/", "\\")
        .split("\\")
        .where((segment) => segment.isNotEmpty)
        .toList();
    return segments.isEmpty ? path : segments.last;
  }
}

class _LibraryCanvas extends StatelessWidget {
  const _LibraryCanvas({
    required this.state,
    required this.onImport,
    required this.onRetry,
    required this.onLoadMore,
  });

  final LibraryState state;
  final VoidCallback onImport;
  final VoidCallback onRetry;
  final VoidCallback onLoadMore;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(32, 28, 32, 18),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      "Library",
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
                    const SizedBox(height: 6),
                    Text(
                      _summaryText(context, state),
                      key: const Key("library-summary"),
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        if (state.isProcessing) const LinearProgressIndicator(minHeight: 2),
        if (_needsBanner(state)) _StatusBanner(state: state, onRetry: onRetry),
        Expanded(
          child: state.assets.isEmpty
              ? _EmptyLibraryState(state: state, onImport: onImport)
              : _GalleryGrid(state: state, onLoadMore: onLoadMore),
        ),
      ],
    );
  }

  static String _summaryText(BuildContext context, LibraryState state) {
    final totalItems = state.roots.fold(
      0,
      (total, root) => total + root.assetCount,
    );
    if (state.roots.isEmpty) {
      return "Add a folder to start your library";
    }
    final formattedTotal = MaterialLocalizations.of(
      context,
    ).formatDecimal(totalItems);
    return "$formattedTotal images";
  }

  static bool _needsBanner(LibraryState state) {
    return state.status == LibraryStatus.failed ||
        state.status == LibraryStatus.cancelled ||
        state.status == LibraryStatus.paused ||
        state.status == LibraryStatus.stale ||
        state.isScanLimited;
  }
}

class _StatusBanner extends StatelessWidget {
  const _StatusBanner({required this.state, required this.onRetry});

  final LibraryState state;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final (icon, message, canRetry) = switch (state.status) {
      LibraryStatus.failed => (
        Icons.error_outline,
        state.errorMessage ?? "The scan failed",
        true,
      ),
      LibraryStatus.cancelled => (
        Icons.pause_circle_outline,
        "The partial scan was not published as the trusted catalog.",
        true,
      ),
      LibraryStatus.paused => (
        Icons.pause_circle_outline,
        "The checkpoint is saved. The staged scan remains private until you resume and it completes.",
        true,
      ),
      LibraryStatus.stale => (
        Icons.update_disabled_outlined,
        "A source file changed during scanning. The staged result was not published.",
        true,
      ),
      _ => (
        Icons.info_outline,
        "This import reached its configured scan limit. The completed results "
            "are available, but this source has not been fully indexed.",
        false,
      ),
    };

    return Material(
      color: Theme.of(context).colorScheme.secondaryContainer,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 12),
        child: Row(
          children: [
            Icon(icon),
            const SizedBox(width: 12),
            Expanded(child: Text(message)),
            if (canRetry)
              TextButton(
                onPressed: onRetry,
                child: Text(
                  state.status == LibraryStatus.paused ? "Resume" : "Retry",
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _EmptyLibraryState extends StatelessWidget {
  const _EmptyLibraryState({required this.state, required this.onImport});

  final LibraryState state;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    final isProcessing = state.isProcessing;
    final isPaused = state.status == LibraryStatus.paused;

    return Center(
      key: const Key("library-empty-state"),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                isPaused
                    ? Icons.pause_circle_outline
                    : isProcessing
                    ? Icons.hourglass_top
                    : Icons.add_photo_alternate_outlined,
                size: 56,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 20),
              Text(
                isPaused
                    ? "Scan paused safely"
                    : isProcessing
                    ? state.isResumingScan
                          ? "Resuming the local catalog"
                          : "Preparing the local catalog"
                    : "Build your photo library",
                style: Theme.of(context).textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              Text(
                "Choose the folders you want to browse together in Ame.",
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              const SizedBox(height: 24),
              FilledButton.icon(
                onPressed: state.isBusy ? null : onImport,
                icon: const Icon(Icons.create_new_folder_outlined),
                label: const Text("Import"),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _GalleryDateGroup {
  const _GalleryDateGroup({required this.dateKey, required this.assets});

  final String? dateKey;
  final List<LibraryAsset> assets;

  static List<_GalleryDateGroup> fromAssets(List<LibraryAsset> assets) {
    final groups = <_GalleryDateGroup>[];
    String? activeKey;
    var activeAssets = <LibraryAsset>[];
    var hasActiveGroup = false;
    for (final asset in assets) {
      final dateKey = _captureDateKey(asset);
      if (hasActiveGroup && dateKey != activeKey) {
        groups.add(
          _GalleryDateGroup(
            dateKey: activeKey,
            assets: List.unmodifiable(activeAssets),
          ),
        );
        activeAssets = <LibraryAsset>[];
      }
      activeKey = dateKey;
      activeAssets.add(asset);
      hasActiveGroup = true;
    }
    if (hasActiveGroup) {
      groups.add(
        _GalleryDateGroup(
          dateKey: activeKey,
          assets: List.unmodifiable(activeAssets),
        ),
      );
    }
    return List.unmodifiable(groups);
  }

  static String? _captureDateKey(LibraryAsset asset) {
    final localTime = asset.captureTime?.localTime;
    if (localTime == null || localTime.length < 10) {
      return null;
    }
    final dateKey = localTime.substring(0, 10);
    final parts = dateKey.split("-");
    if (parts.length != 3 || parts.any((part) => int.tryParse(part) == null)) {
      return null;
    }
    return dateKey;
  }

  String label(BuildContext context) {
    final key = dateKey;
    if (key == null) {
      return "Unknown capture date";
    }
    final parts = key.split("-").map(int.parse).toList(growable: false);
    final date = DateTime(parts[0], parts[1], parts[2]);
    return MaterialLocalizations.of(context).formatFullDate(date);
  }
}

class _GalleryGrid extends StatelessWidget {
  const _GalleryGrid({required this.state, required this.onLoadMore});

  final LibraryState state;
  final VoidCallback onLoadMore;

  @override
  Widget build(BuildContext context) {
    final groups = _GalleryDateGroup.fromAssets(state.assets);
    final hasPageControl =
        state.isLoadingPage || state.pageErrorMessage != null;
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 800 &&
            state.hasMoreAssets &&
            !state.isLoadingPage &&
            state.pageErrorMessage == null) {
          onLoadMore();
        }
        return false;
      },
      child: CustomScrollView(
        key: const Key("library-grid"),
        slivers: [
          for (var index = 0; index < groups.length; index++) ...[
            SliverPadding(
              padding: EdgeInsets.fromLTRB(32, index == 0 ? 18 : 24, 32, 12),
              sliver: SliverToBoxAdapter(
                child: Semantics(
                  header: true,
                  child: Text(
                    groups[index].label(context),
                    key: Key(
                      "gallery-date-${groups[index].dateKey ?? 'unknown'}",
                    ),
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ),
            ),
            SliverPadding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              sliver: SliverGrid.builder(
                gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
                  maxCrossAxisExtent: 260,
                  mainAxisExtent: 220,
                  crossAxisSpacing: 8,
                  mainAxisSpacing: 8,
                ),
                itemCount: groups[index].assets.length,
                itemBuilder: (context, assetIndex) {
                  final asset = groups[index].assets[assetIndex];
                  return _GalleryTile(
                    key: ValueKey(asset.locationId),
                    asset: asset,
                  );
                },
              ),
            ),
          ],
          if (hasPageControl)
            SliverToBoxAdapter(
              child: SizedBox(
                height: 72,
                child: _CatalogPageControl(
                  isLoading: state.isLoadingPage,
                  errorMessage: state.pageErrorMessage,
                  onLoadMore: onLoadMore,
                ),
              ),
            ),
          const SliverPadding(padding: EdgeInsets.only(bottom: 32)),
        ],
      ),
    );
  }
}

class _CatalogPageControl extends StatelessWidget {
  const _CatalogPageControl({
    required this.isLoading,
    required this.errorMessage,
    required this.onLoadMore,
  });

  final bool isLoading;
  final String? errorMessage;
  final VoidCallback onLoadMore;

  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return const Center(
        child: SizedBox.square(
          dimension: 28,
          child: CircularProgressIndicator(strokeWidth: 3),
        ),
      );
    }
    if (errorMessage == null) {
      return const SizedBox.shrink();
    }
    return Center(
      child: OutlinedButton.icon(
        key: const Key("library-load-more-button"),
        onPressed: onLoadMore,
        icon: const Icon(Icons.refresh),
        label: const Text("Retry loading"),
      ),
    );
  }
}

class _GalleryTile extends ConsumerStatefulWidget {
  const _GalleryTile({required this.asset, super.key});

  final LibraryAsset asset;

  @override
  ConsumerState<_GalleryTile> createState() => _GalleryTileState();
}

class _GalleryTileState extends ConsumerState<_GalleryTile> {
  late final LibraryController _controller;

  @override
  void initState() {
    super.initState();
    _controller = ref.read(libraryControllerProvider.notifier);
    _schedulePreview();
  }

  @override
  void didUpdateWidget(covariant _GalleryTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId ||
        oldWidget.asset.previewStatus != widget.asset.previewStatus) {
      _controller.cancelPreview(oldWidget.asset.locationId);
      _schedulePreview();
    }
  }

  @override
  void dispose() {
    _controller.cancelPreview(widget.asset.locationId);
    super.dispose();
  }

  void _schedulePreview() {
    if (widget.asset.previewStatus != LibraryPreviewStatus.pending) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _controller.requestPreview(widget.asset);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final asset = widget.asset;
    return Tooltip(
      message: asset.previewIssueMessage == null
          ? asset.sourcePath
          : "${asset.sourcePath}\n${asset.previewIssueMessage}",
      child: ClipRRect(
        borderRadius: BorderRadius.circular(8),
        child: ColoredBox(
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Stack(
            fit: StackFit.expand,
            children: [
              switch (asset.previewStatus) {
                LibraryPreviewStatus.pending => const Center(
                  key: Key("library-preview-pending"),
                  child: CircularProgressIndicator(strokeWidth: 3),
                ),
                LibraryPreviewStatus.failed => Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.broken_image_outlined),
                      const SizedBox(height: 8),
                      TextButton(
                        key: Key("preview-retry-${asset.locationId}"),
                        onPressed: () {
                          _controller.requestPreview(asset, retry: true);
                        },
                        child: const Text("Retry preview"),
                      ),
                    ],
                  ),
                ),
                LibraryPreviewStatus.ready => Image.file(
                  File(asset.previewPath),
                  fit: BoxFit.cover,
                  cacheWidth: 512,
                  filterQuality: FilterQuality.low,
                  errorBuilder: (context, error, stackTrace) {
                    return const Center(
                      child: Icon(Icons.broken_image_outlined),
                    );
                  },
                ),
              },
            ],
          ),
        ),
      ),
    );
  }
}
