import "dart:async";
import "dart:math" as math;

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_locations_sheet.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  testWidgets("scrolling crosses batches and retries a failed continuation", (
    tester,
  ) async {
    final service = _Locations()..failContinuation = true;
    await tester.pumpWidget(_host(service));
    await tester.pumpAndSettle();
    expect(service.cursors, [null]);
    expect(find.text("200 张重复图片"), findsOneWidget);
    expect(find.text("上一页"), findsNothing);
    expect(find.text("下一页"), findsNothing);

    final list = find.byType(Scrollable);
    await tester.scrollUntilVisible(
      find.text("image-60.png"),
      500,
      scrollable: list,
    );
    await tester.pumpAndSettle();
    expect(service.cursors, [null, "63"]);
    expect(find.text("暂时无法读取结果"), findsOneWidget);
    final offset = tester.state<ScrollableState>(list).position.pixels;
    await tester.pump(const Duration(seconds: 1));
    expect(service.cursors, [null, "63"]);
    service.failContinuation = false;
    await tester.tap(find.text("重试"));
    await tester.pumpAndSettle();
    expect(find.text("暂时无法读取结果"), findsNothing);
    expect(tester.state<ScrollableState>(list).position.pixels, offset);
    await tester.scrollUntilVisible(
      find.text("image-72.png"),
      300,
      scrollable: list,
    );
    await tester.pumpAndSettle();
    expect(find.text("image-72.png"), findsOneWidget);
    expect(
      service.cursors.every((cursor) => cursor == null || cursor == "63"),
      isTrue,
    );
    expect(find.byType(ListTile).evaluate().length, lessThan(30));
    await tester.scrollUntilVisible(
      find.text("image-0.png"),
      -500,
      scrollable: list,
    );
    await tester.pumpAndSettle();
    expect(find.text("image-0.png"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets("replaced and disposed sheets reject pending location reads", (
    tester,
  ) async {
    final first = Completer<LibraryDuplicateLocationPage>();
    final second = Completer<LibraryDuplicateLocationPage>();
    final service = _Locations()..pending = first;
    await tester.pumpWidget(_host(service));
    await tester.pump();
    service.pending = second;
    await tester.pumpWidget(_host(service, group: reviewGroup));
    await tester.pump();
    first.complete(service.page(0));
    await tester.pump();
    expect(find.text("image-0.png"), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
    second.complete(service.page(0));
    await tester.pump();
    expect(tester.takeException(), isNull);
  });
}

Widget _host(_Locations service, {LibraryDuplicateGroup group = _group}) =>
    ProviderScope(
      overrides: [
        libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
        libraryExactDuplicatesProvider.overrideWithValue(service),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: LibraryDuplicateLocationsSheet(
            group: group,
            currentLocationId: "0",
          ),
        ),
      ),
    );

const _group = LibraryDuplicateGroup(
  digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  byteLength: 1024,
  locationCount: 200,
  physicalFileCount: 200,
  representativeLocationId: "0",
  representativePath: r"C:\fixture\image-0.png",
);

class _Locations extends Fake implements LibraryExactDuplicates {
  final cursors = <String?>[];
  bool failContinuation = false;
  Completer<LibraryDuplicateLocationPage>? pending;

  LibraryDuplicateLocationPage page(int start) {
    final end = math.min(start + 64, 200);
    return LibraryDuplicateLocationPage(
      locations: [
        for (var index = start; index < end; index++)
          LibraryDuplicateLocation(
            locationId: "$index",
            displayPath: "C:\\fixture\\image-$index.png",
            availability: LibraryDuplicateAvailability.local,
          ),
      ],
      nextLocationId: end < 200 ? "${end - 1}" : null,
    );
  }

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    cursors.add(after);
    if (pending case final read?) return read.future;
    if (after != null && failContinuation)
      throw StateError("controlled read failure");
    return page(after == null ? 0 : int.parse(after) + 1);
  }
}
