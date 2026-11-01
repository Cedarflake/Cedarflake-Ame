import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_settings_editor.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_model_installation_feedback.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "editing descriptions retains category identity and does not enable analysis",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      await fixture.open(tester);
      await tester.tap(find.text("插画"));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key("classification-category-name")),
        "绘画",
      );
      await tester.enterText(
        find.byKey(const Key("classification-category-descriptions")),
        "an illustration\na drawing",
      );
      await tester.tap(find.widgetWithText(FilledButton, "保存").last);
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, "保存"));
      await tester.pumpAndSettle();
      final saved = fixture.backend.saved.single;
      expect(saved.$1, LibraryClassificationConfigurationAction.save);
      expect(saved.$2.single.id, "drawing");
      expect(saved.$2.single.label, "绘画");
      expect(saved.$2.single.descriptions, ["an illustration", "a drawing"]);
      expect(find.text("智能相册设置已保存"), findsOneWidget);
      expect(tester.takeException(), isNull);
      await fixture.close(tester);
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  testWidgets(
    "first use downloads without directory or category authoring and leaves browsing available",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.backend.configured = false;
      await fixture.open(tester);
      expect(find.text("选择模型目录"), findsNothing);
      expect(find.text("选择运行库目录"), findsNothing);
      expect(find.text("新建分类"), findsNothing);
      expect(find.text("插画"), findsNothing);
      await tester.tap(find.text("下载并启用"));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 250));
      expect(fixture.backend.saved, isEmpty);
      expect(fixture.installation.busy, isTrue);
      expect(
        find.byKey(const Key("classification-settings-editor")),
        findsNothing,
      );
      await tester.tap(find.text("浏览图片"));
      expect(fixture.browseClicks, 1);
      fixture.session.prepared.complete();
      await tester.pumpAndSettle();
      expect(fixture.session.committed, isTrue);
      expect(
        fixture.backend.saved.single.$1,
        LibraryClassificationConfigurationAction.enable,
      );
      expect(fixture.backend.saved.single.$2.single.id, "drawing");
      expect(find.text("已启用智能相册"), findsOneWidget);
      await fixture.close(tester);
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  testWidgets("download cancellation waits for retirement and never enables", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.backend.configured = false;
    await fixture.open(tester);
    await tester.tap(find.text("下载并启用"));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    await tester.tap(find.text("设置"));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    await tester.tap(find.text("取消启用"));
    await tester.pump();
    expect(find.text("正在停止"), findsOneWidget);
    expect(fixture.session.cancelled, isTrue);
    expect(fixture.session.disposed, isFalse);
    fixture.session.prepared.completeError(StateError("Download cancelled"));
    await tester.pumpAndSettle();
    expect(fixture.backend.saved, isEmpty);
    expect(fixture.session.committed, isFalse);
    expect(fixture.session.disposed, isTrue);
    expect(find.text("已取消启用智能相册"), findsOneWidget);
    await fixture.close(tester);
  });

  testWidgets(
    "failed configuration keeps edits and reuses its operation on retry",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.backend.fail = true;
      await fixture.open(tester);
      await tester.tap(find.widgetWithText(FilledButton, "保存"));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key("classification-settings-editor")),
        findsOneWidget,
      );
      final operation = fixture.backend.operationIds.single;
      fixture.backend.fail = false;
      await tester.tap(find.widgetWithText(FilledButton, "保存"));
      await tester.pumpAndSettle();
      expect(fixture.backend.operationIds, [operation, operation]);
      expect(fixture.backend.saved, hasLength(1));
      await fixture.close(tester);
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );
}

class _Fixture {
  final backend = _Configuration();
  final session = _InstallationSession();
  late final installation = LibraryModelInstallation(_Installation(session));
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
                  Row(
                    children: [
                      TextButton(
                        onPressed: () =>
                            showLibraryClassificationSettings(context),
                        child: const Text("设置"),
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
    await tester.tap(find.text("设置"));
    await tester.pumpAndSettle();
  }

  Future<void> close(WidgetTester tester) async {
    await installation.close();
    final closed = lifecycle.close();
    await tester.pump();
    await closed;
  }

  void dispose() {
    lifecycle.dispose();
    reads.dispose();
  }
}

class _Inactive implements LibraryClassification {
  @override
  Future<LibraryClassificationSession?> open() async => null;
}

class _Configuration implements LibraryClassificationConfigurationService {
  bool configured = true;
  bool fail = false;
  final saved =
      <
        (
          LibraryClassificationConfigurationAction,
          List<LibraryClassificationCategory>,
        )
      >[];
  final operationIds = <String>[];

  @override
  Future<LibraryClassificationSettings> load() async =>
      LibraryClassificationSettings(
        revision: BigInt.one,
        dispatchRevision: BigInt.one,
        mode: LibraryClassificationMode.disabled,
        categories: configured ? await initialCategories() : [],
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
    operationIds.add(operationId);
    if (fail) throw StateError("controlled persistence failure");
    saved.add((action, categories));
    return load();
  }

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) => throw UnimplementedError();
}

class _Installation implements LibraryModelInstallationService {
  _Installation(this.session);
  final _InstallationSession session;
  @override
  LibraryModelInstallationSession create() => session;
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
