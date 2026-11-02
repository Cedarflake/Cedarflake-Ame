import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/domain/library_gallery_collection.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_model_installation_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_smart_album_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("restored categories without a model retain download access", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.backend.retainedCategories = true;
    await fixture.open(tester);
    expect(find.text("插画"), findsOneWidget);
    expect(find.text("还没有分类结果"), findsNothing);
    await tester.tap(find.text("下载模型"));
    await tester.pumpAndSettle();
    expect(find.text("下载并启用"), findsOneWidget);
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(fixture.installer.sessions, isEmpty);
    expect(fixture.backend.saved, isEmpty);
    await fixture.close(tester);
  });

  testWidgets("first use requires confirmation before downloading", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    await fixture.open(tester);
    expect(find.text("还没有分类结果"), findsOneWidget);
    await tester.tap(find.text("下载模型"));
    await tester.pumpAndSettle();
    expect(find.textContaining("504 MB"), findsOneWidget);
    expect(find.byType(TextField), findsNothing);
    expect(fixture.installer.sessions, isEmpty);
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(fixture.installer.sessions, isEmpty);
    expect(fixture.backend.saved, isEmpty);
    await fixture.close(tester);
  });

  testWidgets(
    "download leaves browsing available and removes setup after activation",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      await fixture.open(tester);
      await fixture.download(tester);
      expect(find.byType(AlertDialog), findsNothing);
      expect(fixture.backend.saved, isEmpty);
      await tester.tap(find.text("浏览图片"));
      expect(fixture.browseClicks, 1);
      final session = fixture.installer.sessions.single;
      session.prepared.complete();
      await tester.pumpAndSettle();
      expect(session.committed, isTrue);
      expect(session.disposed, isTrue);
      expect(fixture.backend.saved.single.single.id, "drawing");
      expect(find.text("插画"), findsOneWidget);
      expect(find.text("下载模型"), findsNothing);
      expect(find.text("已启用智能相册"), findsOneWidget);
      await fixture.close(tester);
    },
  );

  testWidgets("download cancellation waits for retirement without enabling", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    await fixture.open(tester);
    await fixture.download(tester);
    await tester.tap(find.text("取消"));
    await tester.pump();
    final session = fixture.installer.sessions.single;
    expect(find.text("正在停止"), findsOneWidget);
    expect(session.cancelled, isTrue);
    expect(session.disposed, isFalse);
    session.prepared.completeError(StateError("Download cancelled"));
    await tester.pumpAndSettle();
    expect(session.committed, isFalse);
    expect(session.disposed, isTrue);
    expect(fixture.backend.saved, isEmpty);
    expect(find.text("下载模型"), findsOneWidget);
    await fixture.close(tester);
  });

  testWidgets(
    "failed activation retries through download feedback without settings",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.backend.fail = true;
      await fixture.open(tester);
      await fixture.download(tester);
      fixture.installer.sessions.single.prepared.complete();
      await tester.pumpAndSettle();
      expect(find.text("智能相册未能启用"), findsOneWidget);
      expect(find.text("打开设置"), findsNothing);
      expect(fixture.backend.saved, isEmpty);
      fixture.backend.fail = false;
      await tester.tap(find.text("重试"));
      await tester.pumpAndSettle();
      await tester.tap(find.text("下载并启用"));
      await tester.pump();
      fixture.installer.sessions.last.prepared.complete();
      await tester.pumpAndSettle();
      expect(fixture.backend.saved, hasLength(1));
      expect(find.text("下载模型"), findsNothing);
      expect(find.text("智能相册未能启用"), findsNothing);
      await fixture.close(tester);
    },
  );

  for (final mode in LibraryClassificationMode.values) {
    testWidgets(
      "installed ${mode.name} setup has no settings or status prompt",
      (tester) async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        fixture.backend.configured = true;
        fixture.backend.mode = mode;
        await fixture.open(tester);
        expect(find.text("插画"), findsOneWidget);
        expect(find.text("下载模型"), findsNothing);
        expect(find.text("智能相册设置"), findsNothing);
        expect(find.text("启用"), findsNothing);
        expect(find.byType(AlertDialog), findsNothing);
        expect(fixture.installer.sessions, isEmpty);
        await fixture.close(tester);
      },
    );
  }
}

class _Fixture {
  final backend = _Configuration();
  final installer = _Installation();
  late final installation = LibraryModelInstallation(installer);
  int browseClicks = 0;
  final reads = LibraryForegroundReads();
  late final lifecycle = LibraryClassificationLifecycle(
    _Inactive(),
    foregroundReads: reads,
  );
  late final configuration = LibraryClassificationConfiguration(
    backend,
    lifecycle,
  );

  Future<void> open(WidgetTester tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryClassificationLifecycleProvider.overrideWith(
            (ref) => lifecycle,
          ),
          libraryClassificationConfigurationProvider.overrideWithValue(
            configuration,
          ),
          libraryModelInstallationProvider.overrideWith((ref) => installation),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: Consumer(
              builder: (context, ref, _) => Stack(
                children: [
                  Column(
                    children: [
                      SizedBox(
                        width: 300,
                        child: LibrarySmartAlbumNavigation(
                          collection: const LibraryGalleryCollection.library(),
                          enabled: true,
                          onSelect: (_) {},
                        ),
                      ),
                      TextButton(
                        onPressed: () => browseClicks += 1,
                        child: const Text("浏览图片"),
                      ),
                    ],
                  ),
                  Align(
                    alignment: Alignment.bottomCenter,
                    child:
                        buildLibraryModelInstallationFeedback(context, ref) ??
                        const SizedBox.shrink(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> download(WidgetTester tester) async {
    await tester.tap(find.text("下载模型"));
    await tester.pumpAndSettle();
    await tester.tap(find.text("下载并启用"));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
  }

  Future<void> close(WidgetTester tester) async {
    await installation.close();
    final closed = lifecycle.close();
    await tester.pump();
    await closed;
  }

  void dispose() => reads.dispose();
}

class _Inactive implements LibraryClassification {
  @override
  Future<LibraryClassificationSession?> open() async => null;
}

class _Configuration implements LibraryClassificationConfigurationService {
  bool configured = false;
  bool retainedCategories = false;
  bool fail = false;
  LibraryClassificationMode mode = LibraryClassificationMode.disabled;
  final saved = <List<LibraryClassificationCategory>>[];

  @override
  Future<LibraryClassificationSettings> load() async =>
      LibraryClassificationSettings(
        revision: BigInt.one,
        dispatchRevision: BigInt.one,
        mode: mode,
        categories: configured || retainedCategories
            ? await initialCategories()
            : [],
        runId: null,
        installationSelected: configured,
      );

  @override
  Future<List<LibraryClassificationCategory>> initialCategories() async =>
      const [
        LibraryClassificationCategory(
          id: "drawing",
          label: "插画",
          descriptions: ["an illustration"],
        ),
      ];

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) async {
    if (fail) throw StateError("controlled persistence failure");
    saved.add(categories);
    configured = true;
    mode = LibraryClassificationMode.automatic;
    return load();
  }

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) => throw UnimplementedError();
}

class _Installation implements LibraryModelInstallationService {
  final sessions = <_InstallationSession>[];

  @override
  LibraryModelInstallationSession create() {
    final session = _InstallationSession();
    sessions.add(session);
    return session;
  }
}

class _InstallationSession implements LibraryModelInstallationSession {
  final prepared = Completer<void>();
  bool committed = false;
  bool cancelled = false;
  bool disposed = false;
  @override
  Future<void> prepare() => prepared.future;
  @override
  LibraryModelDownloadProgress progress() => const LibraryModelDownloadProgress(
    downloading: true,
    completedBytes: 50,
    totalBytes: 100,
  );
  @override
  void cancel() => cancelled = true;
  @override
  Future<void> commit() async => committed = true;
  @override
  void dispose() => disposed = true;
}
