import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:flutter/gestures.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

void main() {
  for (final shape in GalleryLayoutShape.values) {
    testWidgets("${shape.name} publishes pages while the rail stays held", (
      tester,
    ) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final requests = <({int offset, Completer<void> completion})>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: _PagedGallery(
              controller: controller,
              shape: shape,
              requests: requests,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final rail = tester.getRect(find.byKey(const Key("timeline-slider")));
      Offset point(double fraction) =>
          Offset(rail.center.dx, rail.top + rail.height * fraction);
      final pointer = await tester.startGesture(
        point(0.05),
        kind: PointerDeviceKind.mouse,
      );
      await pointer.moveTo(point(0.4));
      await tester.pump();
      expect(requests, hasLength(1));
      expect(controller.offset, greaterThan(30000));

      // Keep moving while the first catalog window is still being read.
      for (final fraction in [0.42, 0.44, 0.46]) {
        await pointer.moveTo(point(fraction));
        await tester.pump(const Duration(milliseconds: 40));
      }
      expect(requests, hasLength(1));
      final latestPosition = controller.offset;
      final first = requests.first;
      first.completion.complete();
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(controller.offset, latestPosition);
      final loaded = tester.widget<LibraryTimeNavigation>(
        find.byType(LibraryTimeNavigation),
      );
      expect(loaded.windowStartItemOffset, lessThanOrEqualTo(first.offset));
      expect(
        loaded.windowStartItemOffset + loaded.loadedItemCount,
        greaterThan(first.offset),
      );

      await tester.pump(const Duration(milliseconds: 140));
      expect(requests, hasLength(2));
      final next = requests.last;
      expect(next.offset, greaterThan(first.offset));
      next.completion.complete();
      await tester.pumpAndSettle();
      expect(find.text("Photo ${next.offset}").hitTestable(), findsOneWidget);

      await pointer.moveTo(point(0.8));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 140));
      expect(requests, hasLength(3));
      await pointer.up();
      await tester.pump();
      expect(requests, hasLength(3));
      for (final request in requests) {
        if (!request.completion.isCompleted) request.completion.complete();
      }
      await tester.pumpAndSettle();
      expect(
        find.text("Photo ${requests.last.offset}").hitTestable(),
        findsOneWidget,
      );
    });
  }
}

class _PagedGallery extends StatefulWidget {
  const _PagedGallery({
    required this.controller,
    required this.shape,
    required this.requests,
  });

  final ScrollController controller;
  final GalleryLayoutShape shape;
  final List<({int offset, Completer<void> completion})> requests;

  @override
  State<_PagedGallery> createState() => _PagedGalleryState();
}

class _PagedGalleryState extends State<_PagedGallery> {
  var _timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "gallery",
    totalItems: 1000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 1000,
        aspectRatioSum: 1000,
      ),
    ],
  );
  int _windowStart = 0;
  int _cancellation = 0;
  LibraryTimeAnchor? _resolvedTimeAnchor;

  @override
  Widget build(BuildContext context) {
    final leading = _windowStart * 100.0;
    final trailing = 100000 - leading - 2000;
    return Row(
      children: [
        Expanded(
          child: CustomScrollView(
            controller: widget.controller,
            slivers: [
              SliverToBoxAdapter(child: SizedBox(height: leading)),
              SliverFixedExtentList(
                itemExtent: 100,
                delegate: SliverChildBuilderDelegate(
                  (_, index) => Text("Photo ${_windowStart + index}"),
                  childCount: 20,
                ),
              ),
              SliverToBoxAdapter(child: SizedBox(height: trailing)),
            ],
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.controller,
          layoutMetrics: LibraryGalleryLayoutMetrics(
            contentExtent: 2000,
            photoRowHeight: 100,
            itemIndexBase: _windowStart,
            itemOffsets: List.generate(20, (index) => index * 100.0),
            locationOffsets: const {},
            dateAnchors: const [
              LibraryGalleryDateAnchor(
                id: "2026-08-05",
                label: "2026年8月5日",
                scrollOffset: 0,
                year: 2026,
                isUnknown: false,
              ),
            ],
          ),
          timeline: _timeline,
          resolvedTimeAnchor: _resolvedTimeAnchor,
          layoutShape: widget.shape,
          virtualGeometry: LibraryVirtualGalleryGeometry(
            totalContentExtent: 100000,
            viewportExtent: 600,
            leadingExtent: leading,
            loadedContentExtent: 2000,
            trailingExtent: trailing,
            windowStartItemOffset: _windowStart,
            loadedItemCount: 20,
            totalItemCount: 1000,
            queryId: "gallery",
          ),
          windowStartItemOffset: _windowStart,
          loadedItemCount: 20,
          onCancelSeek: () => _cancellation++,
          onSeek: (_, offset) async {
            final cancellation = _cancellation;
            final completion = Completer<void>();
            widget.requests.add((offset: offset, completion: completion));
            await completion.future;
            if (!mounted || cancellation != _cancellation) return false;
            setState(() {
              _windowStart = (offset - 3).clamp(0, 980);
              _timeline = LibraryTimeline(
                revision: _timeline.revision + BigInt.one,
                queryId: _timeline.queryId,
                totalItems: _timeline.totalItems,
                buckets: _timeline.buckets,
              );
              _resolvedTimeAnchor = LibraryTimeAnchor(
                revision: _timeline.revision,
                queryId: _timeline.queryId,
                monthKey: "2026-08",
                itemOffset: offset,
              );
            });
            return true;
          },
        ),
      ],
    );
  }
}
