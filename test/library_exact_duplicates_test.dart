import "dart:async";

import "package:cedarflake_ame/features/library/application/library_duplicate_results_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:cedarflake_ame/features/library/application/library_workbench_controller.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

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
  Completer<LibraryDuplicateGroupPage>? delayedGroups;
  Completer<LibraryDuplicateLocationPage>? delayedLocations;

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
    if (delayedGroups case final pending?) {
      return pending.future;
    }
    return LibraryDuplicateGroupPage(groups: hasResults ? [_group] : []);
  }

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    if (delayedLocations case final pending?) {
      return pending.future;
    }
    return const LibraryDuplicateLocationPage(
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

  test("a late group response cannot clear a newer location request", () async {
    final service = _Service()
      ..delayedGroups = Completer<LibraryDuplicateGroupPage>()
      ..delayedLocations = Completer<LibraryDuplicateLocationPage>();
    final controller = LibraryDuplicateResultsController(service);
    final groups = controller.refresh();
    final locations = controller.showLocations(_group);
    service.delayedGroups!.complete(
      const LibraryDuplicateGroupPage(groups: [_group]),
    );
    await groups;
    expect(controller.selected, _group);
    expect(controller.loading, isTrue);
    service.delayedLocations!.complete(
      const LibraryDuplicateLocationPage(locations: []),
    );
    await locations;
    expect(controller.loading, isFalse);
    controller.dispose();
    await service.events.close();
  });

  testWidgets(
    "workspace switches retain analysis and locations reveal the selected file",
    (tester) async {
      final service = _Service();
      final revealed = <List<String>>[];
      final container = ProviderContainer(
        overrides: [
          libraryExactDuplicatesProvider.overrideWithValue(service),
          libraryPlatformActionsProvider.overrideWithValue(
            WindowsLibraryPlatformActions.withLauncher((arguments) async {
              revealed.add(arguments);
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
                navigationBuilder: (width) => SizedBox(width: width),
                galleryBuilder: (toggle) =>
                    Column(children: [toggle, const Text("Gallery")]),
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
      await tester.tap(find.widgetWithText(TextButton, "查看重复位置"));
      await tester.pumpAndSettle();
      expect(find.text(r"C:\fixture\one.png"), findsOneWidget);
      expect(find.text(r"D:\fixture\two.png"), findsOneWidget);
      expect(find.text("文件夹离线"), findsOneWidget);
      await tester.tap(find.text(r"C:\fixture\one.png"));
      await tester.pumpAndSettle();
      expect(revealed, [
        ["/select,", r"C:\fixture\one.png"],
      ]);
      final offline = tester.widget<ListTile>(
        find.byKey(const ValueKey("duplicate-location-two")),
      );
      expect(offline.onTap, isNull);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}
