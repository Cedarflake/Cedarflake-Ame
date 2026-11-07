import "dart:math" as math;

import "package:flutter/material.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_brand_mark.dart";
import "../../../../app/presentation/ame_notifications.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/window/ame_window_chrome.dart";
import "../library_strings.dart";
import "../coordination/library_search.dart";
import "library_search_bar.dart";
import "library_workbench_metrics.dart";

class LibraryGlobalBar extends StatelessWidget {
  const LibraryGlobalBar({
    required this.isSearchEnabled,
    required this.searchController,
    required this.sidebarWidth,
    required this.sidebarExpansion,
    this.onSearchChanged,
    this.discovery,
    required this.notifications,
    required this.onNotificationsOpened,
    required this.onNotificationSelected,
    super.key,
  });

  final bool isSearchEnabled;
  final TextEditingController searchController;
  final double sidebarWidth;
  final double sidebarExpansion;
  final ValueChanged<String>? onSearchChanged;
  final LibrarySearch? discovery;
  final AmeNotificationState notifications;
  final VoidCallback onNotificationsOpened;
  final ValueChanged<AmeNotificationEntry> onNotificationSelected;

  @override
  Widget build(BuildContext context) {
    return Material(
      key: const Key("library-global-surface"),
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      child: SizedBox(
        key: const Key("library-global-bar"),
        height: 64,
        child: LayoutBuilder(
          builder: (context, constraints) {
            const collapsedLeading =
                LibraryWorkbenchMetrics.railWidth +
                LibraryWorkbenchMetrics.galleryHeadingInset;
            final contentLeading =
                collapsedLeading + sidebarWidth * sidebarExpansion;
            final leadingWidth = (contentLeading - 16).clamp(
              LibraryWorkbenchMetrics.railWidth,
              244.0,
            );
            final isCompact = leadingWidth < 244;
            const captionWidth = 48.0 * 3 + 8;
            const notificationWidth = 48.0;
            const notificationGap = 8.0;
            const collapsedSearchGap = 16.0;
            const expandedSearchGap = 32.0;
            const trailingWidth =
                captionWidth + notificationWidth + notificationGap;
            final collapsedSearchWidth = math.max(
              0.0,
              constraints.maxWidth -
                  collapsedLeading -
                  trailingWidth -
                  collapsedSearchGap,
            );
            final expandedSearchWidth = math.min(
              720.0,
              math.max(
                0.0,
                constraints.maxWidth -
                    collapsedLeading -
                    sidebarWidth -
                    trailingWidth -
                    expandedSearchGap,
              ),
            );
            // Fixed endpoints keep the right edge from reversing mid-transition.
            final searchWidth =
                collapsedSearchWidth +
                (expandedSearchWidth - collapsedSearchWidth) * sidebarExpansion;
            return Stack(
              fit: StackFit.expand,
              children: [
                const Positioned.fill(
                  child: AmeWindowDragRegion(child: SizedBox.expand()),
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: AmeWindowDragRegion(
                    child: SizedBox(
                      width: leadingWidth,
                      child: Padding(
                        padding: const EdgeInsets.only(
                          left: LibraryWorkbenchMetrics.railIconInset,
                        ),
                        child: Row(
                          children: [
                            Semantics(
                              label: isCompact ? LibraryStrings.appName : null,
                              image: isCompact,
                              child: const AmeBrandMark(
                                size: LibraryWorkbenchMetrics.iconSize,
                              ),
                            ),
                            if (!isCompact) ...[
                              const SizedBox(width: 12),
                              Flexible(
                                child: Text(
                                  LibraryStrings.appName,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.primary,
                                    fontWeight: ameFontWeightSemibold,
                                  ),
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: contentLeading,
                  top: 0,
                  bottom: 0,
                  child: SizedBox(
                    width: searchWidth,
                    child: Align(
                      key: const Key("library-search-alignment"),
                      child: LibrarySearchBar(
                        controller: searchController,
                        enabled: isSearchEnabled,
                        discovery: discovery,
                        onChanged: onSearchChanged,
                      ),
                    ),
                  ),
                ),
                Positioned(
                  right: captionWidth + notificationGap,
                  top: 8,
                  child: SizedBox.square(
                    dimension: notificationWidth,
                    child: AmeNotificationHistoryButton(
                      state: notifications,
                      onOpened: onNotificationsOpened,
                      onSelected: onNotificationSelected,
                    ),
                  ),
                ),
                const Align(
                  alignment: Alignment.centerRight,
                  child: AmeWindowCaptionControls(height: 64),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
