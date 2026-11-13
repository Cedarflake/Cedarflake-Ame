import "dart:async";
import "dart:math" as math;

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/ports/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_locations_sheet.dart";
import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";
import "support/gallery_group_fixtures.dart";

import "support/application_test_bindings.dart";

void main() {
  testWidgets("verification refreshes coalesce while retaining current rows", (
    tester,
  ) async {
    final events = StreamController<LibraryExactAnalysisProgress>();
    final service = _Locations()
      ..pendingCount = 2
      ..analysisEvents = events.stream;
    final library = _RevisionLibrary();
    final analysis = LibraryExactAnalysisController(service);
    final scheduler = LibraryExactAnalysisScheduler(analysis);
    addTearDown(() async {
      scheduler.dispose();
      analysis.dispose();
      await events.close();
    });
    await tester.pumpWidget(
      _host(
        service,
        library: library,
        analysis: analysis,
        scheduler: scheduler,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text("200 张重复图片"), findsOneWidget);
    expect(find.text("等待后台"), findsOneWidget);
    expect(find.text("当前图片"), findsNothing);
    final deleteSelected = find.widgetWithText(TextButton, "删除所选");
    expect(tester.widget<TextButton>(deleteSelected).onPressed, isNull);
    await tester.tap(find.text("image-0.png"));
    await tester.pumpAndSettle();
    expect(tester.widget<TextButton>(deleteSelected).onPressed, isNotNull);
    scheduler.setPaused(true);
    await tester.pumpAndSettle();
    expect(find.text("已暂停"), findsOneWidget);
    scheduler.setPaused(false);
    analysis.start();
    await tester.pump();
    expect(find.text("正在更新"), findsOneWidget);
    expect(find.text("等待后台"), findsNothing);
    final oldRead = Completer<LibraryDuplicateLocationPage>();
    service.pending = oldRead;
    library.publishRevision(1);
    await tester.pump();
    final latestRead = Completer<LibraryDuplicateLocationPage>();
    service.pending = latestRead;
    library.publishRevision(2);
    await tester.pump();
    library.publishRevision(3);
    await tester.pump();
    expect(service.cursors, [null, null]);
    expect(find.text("image-0.png"), findsOneWidget);
    oldRead.complete(service.page(0));
    await tester.pumpAndSettle();
    expect(service.cursors, [null, null, null]);
    expect(find.text("200 张重复图片"), findsOneWidget);
    expect(find.text("正在更新"), findsOneWidget);
    expect(find.text("image-0.png"), findsOneWidget);
    service.pendingCount = 0;
    latestRead.complete(service.page(0));
    await tester.pumpAndSettle();
    expect(find.text("200 张重复图片"), findsOneWidget);
    expect(find.text("正在更新"), findsNothing);
    expect(find.text("image-0.png"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

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
    expect(find.byType(AmeLoadingIndicator), findsWidgets);
    await tester.pumpWidget(const SizedBox());
    second.complete(service.page(0));
    await tester.pump();
    expect(tester.takeException(), isNull);
  });
}

Widget _host(
  _Locations service, {
  LibraryDuplicateGroup group = _group,
  ReviewFixtureLibrary? library,
  LibraryExactAnalysisController? analysis,
  LibraryExactAnalysisScheduler? scheduler,
}) => ProviderScope(
  overrides: applicationTestOverrides([
    libraryControllerProvider.overrideWith(
      () => library ?? ReviewFixtureLibrary(),
    ),
    libraryExactDuplicatesProvider.overrideWithValue(service),
    libraryGroupMemberCatalogProvider.overrideWithValue(
      LocationPageGroupCatalog((after) => service.loadLocations(group, after)),
    ),
    if (analysis != null)
      libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
    if (scheduler != null)
      libraryExactAnalysisSchedulerProvider.overrideWithValue(scheduler),
  ]),
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
  Stream<LibraryExactAnalysisProgress> analysisEvents = const Stream.empty();
  final cursors = <String?>[];
  bool failContinuation = false;
  int pendingCount = 0;
  Completer<LibraryDuplicateLocationPage>? pending;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) => analysisEvents;

  @override
  Future<void> cancel(String runId) async {}

  LibraryDuplicateLocationPage page(int start) {
    final end = math.min(start + 64, 200);
    return LibraryDuplicateLocationPage(
      locationCount: 200,
      pendingLocationCount: pendingCount,
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
    if (after != null && failContinuation) {
      throw StateError("controlled read failure");
    }
    return page(after == null ? 0 : int.parse(after) + 1);
  }
}

class _RevisionLibrary extends ReviewFixtureLibrary {
  void publishRevision(int revision) {
    state = state.copyWith(catalogRevision: BigInt.from(revision));
  }
}
