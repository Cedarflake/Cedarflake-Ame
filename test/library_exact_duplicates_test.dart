import "dart:async";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "support/duplicate_review_fixtures.dart";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_badge.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_analysis_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:cedarflake_ame/features/library/application/library_workbench_controller.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter/gestures.dart";
import "package:flutter/services.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/semantics/retained_semantics_update_harness.dart";

const _group = LibraryDuplicateGroup(
  digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  byteLength: 1024,
  locationCount: 2,
  physicalFileCount: 2,
  representativeLocationId: "one",
  representativePath: r"C:\fixture\one.png",
);

class _Service implements LibraryExactDuplicates {
  final events = StreamController<LibraryExactAnalysisProgress>.broadcast();
  String? runId;
  int cancellations = 0;
  bool hasResults = false;
  bool hasLocations = true;
  LibraryDuplicateLocationPage? locationPage;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    this.runId = runId;
    return events.stream;
  }

  void progress(LibraryExactAnalysisPhase phase) {
    events.add(
      LibraryExactAnalysisProgress(
        runId: runId!,
        phase: phase,
        checkedFiles: 2,
        fingerprintedFiles: 2,
        skippedFiles: 0,
      ),
    );
  }

  @override
  Future<void> cancel(String runId) async {
    cancellations++;
  }

  @override
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  ) async {
    return LibraryDuplicateGroupPage(groups: hasResults ? [_group] : []);
  }

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    if (!hasLocations) {
      return const LibraryDuplicateLocationPage(locations: []);
    }
    return locationPage ??
        const LibraryDuplicateLocationPage(
          locations: [
            LibraryDuplicateLocation(
              locationId: "one",
              displayPath: r"C:\fixture\one.png",
              availability: LibraryDuplicateAvailability.local,
            ),
            LibraryDuplicateLocation(
              locationId: "two",
              displayPath: r"D:\fixture\two.png",
              availability: LibraryDuplicateAvailability.offline,
            ),
          ],
        );
  }
}

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);
  testWidgets(
    "photo badge opens current paths and reveals the selected location",
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final service = _Service();
        addTearDown(service.events.close);
        final revealed = <String>[];
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              libraryControllerProvider.overrideWith(
                () => ReviewFixtureLibrary(
                  duplicateMode: LibraryGalleryDuplicateMode.merged,
                ),
              ),
              libraryGalleryDuplicateAnnotationsProvider.overrideWith(
                (ref) async => {"one": _group},
              ),
              libraryExactDuplicatesProvider.overrideWithValue(service),
              libraryDuplicateReviewProvider.overrideWithValue(
                ReviewFixtureService(showResults: () => service.hasResults),
              ),
              libraryPlatformActionsProvider.overrideWithValue(
                WindowsLibraryPlatformActions.withRevealer(
                  (path) async => revealed.add(path),
                ),
              ),
            ],
            child: const MaterialApp(
              home: Scaffold(body: LibraryDuplicateBadge(locationId: "one")),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey("duplicate-badge-one")));
        await tester.pumpAndSettle();
        expect(find.text("two.png"), findsOneWidget);
        expect(find.text(r"D:\fixture"), findsOneWidget);
        expect(find.text("当前图片"), findsOneWidget);
        expect(find.textContaining("本地可用"), findsNothing);
        expect(find.byType(BottomSheet), findsOneWidget);
        expect(find.text("关闭"), findsNothing);
        final currentRow = find.byKey(const ValueKey("duplicate-location-one"));
        expect(
          find.descendant(
            of: find.byType(ListTile),
            matching: find.byType(IconButton),
          ),
          findsNothing,
        );
        expect(
          tester.getTopLeft(find.text("当前图片")).dx,
          greaterThan(tester.getTopRight(find.text("one.png")).dx),
        );
        expect(
          tester
              .widget<ListTile>(
                find.byKey(const ValueKey("duplicate-location-two")),
              )
              .trailing,
          isNull,
        );
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer();
        await mouse.moveTo(tester.getCenter(currentRow));
        await tester.pump(const Duration(milliseconds: 600));
        expect(find.text("在文件资源管理器中打开"), findsNothing);
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "duplicate-location-row-hover",
        );
        await mouse.removePointer();
        await tester.tap(find.text("one.png"));
        await tester.pumpAndSettle();
        expect(revealed, [r"C:\fixture\one.png"]);
        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.byType(BottomSheet), findsNothing);
        expect(
          find.byKey(const ValueKey("duplicate-badge-one")),
          findsOneWidget,
        );
        expect(find.text("查看重复图片 · 2 个位置"), findsNothing);
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "duplicate-location-sheet-closed",
        );
      } finally {
        semantics.dispose();
      }
    },
  );
  for (final size in [const Size(1000, 760), const Size(400, 600)]) {
    testWidgets(
      "location sheet wraps paths at $size and returns keyboard focus",
      (tester) async {
        const folder =
            r"C:\fixture\图库\2026\十月\整理后的图片\different-image-sizes\6000x4000\an-extra-long-folder-without-spaces";
        const name = "03325-6000x4000-0313-with-a-long-original-filename.jpg";
        final service = _Service()
          ..locationPage = const LibraryDuplicateLocationPage(
            locations: [
              LibraryDuplicateLocation(
                locationId: "one",
                displayPath: "$folder\\$name",
                availability: LibraryDuplicateAvailability.local,
              ),
            ],
          );
        addTearDown(service.events.close);
        tester.view.devicePixelRatio = 1;
        tester.view.physicalSize = size;
        addTearDown(tester.view.reset);
        var outsideTaps = 0;
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              libraryControllerProvider.overrideWith(
                () => ReviewFixtureLibrary(
                  duplicateMode: LibraryGalleryDuplicateMode.merged,
                ),
              ),
              libraryGalleryDuplicateAnnotationsProvider.overrideWith(
                (_) async => {"one": _group},
              ),
              libraryExactDuplicatesProvider.overrideWithValue(service),
              libraryDuplicateReviewProvider.overrideWithValue(
                ReviewFixtureService(showResults: () => service.hasResults),
              ),
            ],
            child: MaterialApp(
              builder: (context, child) => MediaQuery(
                data: MediaQuery.of(context).copyWith(
                  textScaler: TextScaler.linear(size.width < 500 ? 2 : 1),
                ),
                child: child!,
              ),
              home: Scaffold(
                body: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () => outsideTaps++,
                  child: const Align(
                    alignment: Alignment.bottomRight,
                    child: Padding(
                      padding: EdgeInsets.all(20),
                      child: LibraryDuplicateBadge(locationId: "one"),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final badge = find.byKey(const ValueKey("duplicate-badge-one"));
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pump();
        expect(tester.widget<IconButton>(badge).focusNode!.hasFocus, isTrue);
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.pumpAndSettle();
        final panel = find.byKey(const Key("duplicate-locations-sheet"));
        expect(find.text("1 张重复图片"), findsOneWidget);
        final bounds = tester.getRect(panel);
        expect(bounds.left, greaterThanOrEqualTo(0));
        expect(bounds.right, lessThanOrEqualTo(size.width));
        expect(bounds.top, greaterThanOrEqualTo(0));
        expect(bounds.bottom, lessThanOrEqualTo(size.height));
        expect(bounds.center.dx, size.width / 2);
        expect(bounds.bottom, size.height);
        expect(
          find.byKey(const Key("duplicate-locations-close")),
          findsNothing,
        );
        expect(find.text("上一页"), findsNothing);
        expect(find.text("下一页"), findsNothing);
        for (final text in [name, folder]) {
          final finder = find.text(text);
          final paragraph = tester.renderObject<RenderParagraph>(finder);
          expect(paragraph.didExceedMaxLines, isFalse);
          final textBounds = tester.getRect(finder);
          expect(textBounds.left, greaterThanOrEqualTo(bounds.left));
          expect(textBounds.right, lessThanOrEqualTo(bounds.right));
        }
        expect(find.text("当前图片"), findsOneWidget);
        expect(
          tester.getTopLeft(find.text("当前图片")).dx,
          greaterThan(tester.getTopRight(find.text(folder)).dx),
        );
        await tester.ensureVisible(find.text("当前图片"));
        await tester.pumpAndSettle();
        expect(
          tester.getBottomRight(find.text("当前图片")).dy,
          lessThanOrEqualTo(bounds.bottom),
        );
        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(panel, findsNothing);
        expect(tester.widget<IconButton>(badge).focusNode!.hasFocus, isTrue);
        await tester.tap(badge);
        await tester.pumpAndSettle();
        await tester.tapAt(const Offset(5, 5));
        await tester.pumpAndSettle();
        expect(panel, findsNothing);
        expect(outsideTaps, 0);
        expect(tester.takeException(), isNull);
      },
    );
  }
  for (final saveFails in [false, true]) {
    testWidgets(
      "pause controls persist or restore their state (failure=$saveFails)",
      (tester) async {
        final service = _Service();
        final analysis = LibraryExactAnalysisController(service);
        final scheduler = LibraryExactAnalysisScheduler(analysis, paused: true);
        final store = _PauseStore(saveFails: saveFails);
        final container = ProviderContainer(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryExactDuplicatesProvider.overrideWithValue(service),
            libraryDuplicateReviewProvider.overrideWithValue(
              ReviewFixtureService(showResults: () => service.hasResults),
            ),
            libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
            libraryExactAnalysisSchedulerProvider.overrideWithValue(scheduler),
            initialAmePreferencesProvider.overrideWithValue(store.saved),
            amePreferenceStoreProvider.overrideWithValue(store),
          ],
        );
        addTearDown(() {
          container.dispose();
          scheduler.dispose();
          analysis.dispose();
          unawaited(service.events.close());
        });
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: const MaterialApp(
              home: LibraryAnalysisActivity(
                child: Scaffold(body: LibraryExactDuplicatesPage()),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text("已暂停"), findsNothing);
        expect(find.widgetWithText(TextButton, "继续查重"), findsOneWidget);
        await tester.tap(find.widgetWithText(TextButton, "继续查重"));
        await tester.pumpAndSettle();
        expect(scheduler.isPaused, saveFails);
        expect(
          container.read(amePreferencesControllerProvider).exactAnalysisPaused,
          saveFails,
        );
        if (saveFails) {
          expect(store.saved.exactAnalysisPaused, isTrue);
          expect(find.text("无法保存查重设置"), findsOneWidget);
          expect(find.text("已暂停"), findsNothing);
          expect(find.widgetWithText(TextButton, "继续查重"), findsOneWidget);
        } else {
          expect(store.saved.exactAnalysisPaused, isFalse);
          await tester.tap(find.widgetWithText(TextButton, "暂停查重"));
          await tester.pumpAndSettle();
          expect(scheduler.isPaused, isTrue);
          expect(store.saved.exactAnalysisPaused, isTrue);
          expect(find.text("已暂停"), findsNothing);
          expect(find.widgetWithText(TextButton, "继续查重"), findsOneWidget);
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  test(
    "stopping before native admission is delivered after the first progress event",
    () async {
      final service = _Service();
      final controller = LibraryExactAnalysisController(service);
      controller.start();
      await controller.stop();
      expect(controller.isStopping, isTrue);
      expect(service.cancellations, 0);
      service.progress(LibraryExactAnalysisPhase.running);
      await Future<void>.delayed(Duration.zero);
      expect(service.cancellations, 1);
      service.progress(LibraryExactAnalysisPhase.cancelled);
      await service.events.close();
      expect(controller.isRunning, isFalse);
      expect(controller.progress?.phase, LibraryExactAnalysisPhase.cancelled);
      controller.dispose();
    },
  );

  testWidgets(
    "workspace switches retain analysis and locations reveal the selected file",
    (tester) async {
      final service = _Service();
      final revealed = <String>[];
      final container = ProviderContainer(
        overrides: [
          libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
          libraryExactDuplicatesProvider.overrideWithValue(service),
          libraryDuplicateReviewProvider.overrideWithValue(
            ReviewFixtureService(showResults: () => service.hasResults),
          ),
          libraryPlatformActionsProvider.overrideWithValue(
            WindowsLibraryPlatformActions.withRevealer((path) async {
              revealed.add(path);
            }),
          ),
        ],
      );
      addTearDown(container.dispose);
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.duplicates);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(
              body: LibraryWorkbench(
                albumsNavigationBuilder: (_) => const Text("Album list"),
                navigationBuilder: (width) => SizedBox(width: width),
                galleryBuilder: (_) => const Text("Gallery"),
                duplicatesBuilder: (_) => const LibraryExactDuplicatesPage(),
                settingsBuilder: (_) => const Text("Settings"),
                onSidebarResizeChanged: (_) {},
                onPreferenceError: (_, _) {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, "查找重复图片"));
      service.progress(LibraryExactAnalysisPhase.running);
      await tester.pump();
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.gallery);
      await tester.pumpAndSettle();
      expect(find.text("Gallery"), findsOneWidget);
      expect(service.cancellations, 0);
      service.hasResults = true;
      service.progress(LibraryExactAnalysisPhase.completed);
      await service.events.close();
      await tester.pump();
      container
          .read(libraryWorkbenchControllerProvider.notifier)
          .select(LibraryWorkspace.duplicates);
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(TextButton, "查看重复图片"));
      await tester.pumpAndSettle();
      expect(find.text("one.png"), findsNWidgets(2));
      expect(find.text("two.png"), findsOneWidget);
      expect(find.text("文件夹离线"), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey("reveal-one")));
      await tester.pumpAndSettle();
      expect(revealed, [r"C:\fixture\one.png"]);
      final offline = tester.widget<IconButton>(
        find.byKey(const ValueKey("reveal-two")),
      );
      expect(offline.onPressed, isNull);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}

class _PauseStore implements AmePreferenceStore {
  _PauseStore({required this.saveFails});
  final bool saveFails;
  AmePreferences saved = const AmePreferences(exactAnalysisPaused: true);

  @override
  Future<AmePreferences> loadAmePreferences() async => saved;

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    if (saveFails) {
      throw StateError("Controlled preference write failure");
    }
    saved = preferences;
  }
}
