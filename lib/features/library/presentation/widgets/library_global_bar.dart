import "dart:math" as math;

import "package:material_ui/material_ui.dart";

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
  final ValueChanged<String>? onSearchChanged;
  final LibrarySearch? discovery;
  final AmeNotificationState notifications;
  final VoidCallback onNotificationsOpened;
  final ValueChanged<AmeNotificationEntry> onNotificationSelected;

  @override
  Widget build(BuildContext context) {
    return Material(
      key: const Key("library-global-surface"),
      type: MaterialType.transparency,
      child: SizedBox(
        key: const Key("library-global-bar"),
        height: LibraryWorkbenchMetrics.globalBarHeight,
        child: LayoutBuilder(
          builder: (context, constraints) {
            const brandWidth = 172.0;
            const captionWidth = 48.0 * 3 + 8;
            const notificationWidth = 48.0;
            const notificationGap = 8.0;
            const searchGap = 32.0;
            const trailingWidth =
                captionWidth + notificationWidth + notificationGap;
            final searchWidth = math.min(
              720.0,
              math.max(
                0.0,
                constraints.maxWidth - sidebarWidth - trailingWidth - searchGap,
              ),
            );
            return Stack(
              fit: StackFit.expand,
              children: [
                const Positioned.fill(
                  child: AmeWindowDragRegion(child: SizedBox.expand()),
                ),
                Positioned(
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: brandWidth,
                  child: AmeWindowDragRegion(
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: Padding(
                        padding: const EdgeInsets.only(
                          // The asset's transparent inset aligns its visible edge with the action.
                          left: 10,
                        ),
                        child: Row(
                          children: [
                            Semantics(
                              label: LibraryStrings.appName,
                              image: true,
                              child: const AmeBrandMark(size: 36),
                            ),
                            const SizedBox(width: 4),
                            Flexible(
                              child: Text(
                                LibraryStrings.workbenchName,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontSize: 20,
                                  color: Theme.of(
                                    context,
                                  ).colorScheme.onSurface,
                                  fontWeight: ameFontWeightRegular,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: sidebarWidth,
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
                  child: AmeWindowCaptionControls(
                    height: LibraryWorkbenchMetrics.globalBarHeight,
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
