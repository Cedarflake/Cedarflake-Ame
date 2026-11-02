import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/features/library/application/library_classification.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/domain/library_gallery_collection.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_model_installation_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_smart_album_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/library_model_installation_notifications.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "recovery completed before mount still retains its notification",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.installer.pending = true;
      final recovered = fixture.installation.recover((commit) => commit());
      await tester.pump();
      fixture.installer.sessions.single.prepared.complete();
      await tester.pump();
      await recovered;
      await fixture.open(tester);
      expect(fixture.notifications.history.single.title, "已启用智能相册");
      await fixture.close(tester);
    },
  );
  testWidgets(
    "startup recovery publishes to notification history from the app owner",
    (tester) async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.installer.pending = true;
      final recovering = fixture.installation.recover((commit) => commit());
      await fixture.open(tester);
      expect(fixture.notifications.history, hasLength(1));
      fixture.installer.sessions.single.prepared.complete();
      await tester.pumpAndSettle();
      await recovering;
      expect(fixture.notifications.history.single.title, "已启用智能相册");
      await fixture.close(tester);
    },
  );
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
    expect(
      tester
          .renderObject<RenderParagraph>(find.text("还没有分类结果"))
          .didExceedMaxLines,
      isFalse,
    );
    await tester.tap(find.byTooltip("下载模型"));
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
    expect(find.byTooltip("下载模型"), findsOneWidget);
    await fixture.close(tester);
  });

  testWidgets("hidden download stays live in existing notification history", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    await fixture.open(tester);
    await fixture.download(tester);
    final session = fixture.installer.sessions.single;
    expect(find.text("50% · 0.0 / 0.0 MB"), findsOneWidget);
    await tester.tap(find.text("隐藏"));
    await tester.pump();
    expect(
      find.byKey(const Key("classification-installation-progress")),
      findsNothing,
    );
    expect(session.cancelled, isFalse);
    await tester.tap(find.byKey(const Key("notification-history-button")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("正在下载智能相册模型"));
    await tester.pumpAndSettle();
    session.completedBytes = 75;
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text("75% · 0.0 / 0.0 MB"), findsOneWidget);
    await tester.tap(find.text("取消"));
    await tester.pump();
    expect(session.cancelled, isTrue);
    session.prepared.completeError(StateError("Download cancelled"));
    await tester.pumpAndSettle();
    expect(fixture.notifications.history, hasLength(1));
    expect(fixture.notifications.history.single.title, "已取消启用智能相册");
    expect(fixture.notifications.pendingIds, isEmpty);
    expect(fixture.backend.saved, isEmpty);
    await fixture.close(tester);
  });

  testWidgets("hidden preparation can finish without reopening feedback", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    await fixture.open(tester);
    await fixture.download(tester);
    final session = fixture.installer.sessions.single;
    session.downloading = false;
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text("正在校验模型"), findsOneWidget);
    await tester.tap(find.text("隐藏"));
    await tester.pump();
    session.prepared.complete();
    await tester.pumpAndSettle();
    expect(fixture.backend.saved, hasLength(1));
    expect(find.text("已启用智能相册"), findsNothing);
    expect(fixture.notifications.history.single.title, "已启用智能相册");
    expect(fixture.notifications.history.single.isActive, isFalse);
    expect(fixture.notifications.pendingIds, isEmpty);
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
  late AmeNotificationState notifications;
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
        child: _NotificationOwner(
          child: MaterialApp(
            home: Scaffold(
              body: Consumer(
                builder: (context, ref, _) {
                  notifications = ref.watch(ameNotificationControllerProvider);
                  return Stack(
                    children: [
                      Align(
                        alignment: Alignment.topRight,
                        child: AmeNotificationHistoryButton(
                          state: notifications,
                          onOpened: () => ref
                              .read(ameNotificationControllerProvider.notifier)
                              .markAllRead(),
                          onSelected: (entry) => showAmeNotificationDetails(
                            context,
                            entry,
                            onAction: (observed) =>
                                handleLibraryModelInstallationNotification(
                                  context,
                                  ref,
                                  observed,
                                ),
                          ),
                        ),
                      ),
                      Column(
                        children: [
                          SizedBox(
                            width: 244,
                            child: LibrarySmartAlbumNavigation(
                              collection:
                                  const LibraryGalleryCollection.library(),
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
                            buildLibraryModelInstallationFeedback(
                              context,
                              ref,
                            ) ??
                            const SizedBox.shrink(),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> download(WidgetTester tester) async {
    await tester.tap(find.byTooltip("下载模型"));
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

class _NotificationOwner extends ConsumerWidget {
  const _NotificationOwner({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryModelInstallationNotificationsProvider);
    return child;
  }
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
  Future<LibraryClassificationSettings> loadNavigation() => load();

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
  bool pending = false;

  @override
  Future<LibraryModelInstallationSession?> recover() async =>
      pending ? create() : null;

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
  bool downloading = true;
  int completedBytes = 50;
  @override
  Future<void> prepare() => prepared.future;
  @override
  LibraryModelDownloadProgress progress() => LibraryModelDownloadProgress(
    downloading: downloading,
    completedBytes: completedBytes,
    totalBytes: 100,
  );
  @override
  void cancel() => cancelled = true;
  @override
  Future<void> commit() async => committed = true;
  @override
  Future<void> discard() async {}
  @override
  void dispose() => disposed = true;
}
