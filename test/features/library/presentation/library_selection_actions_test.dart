import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/application/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/application/library_organization_outcome.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_selection_actions.dart";
import "package:cedarflake_ame/features/settings/application/album_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_batch_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationBatchesFixture batches;
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  late ValueNotifier<bool> visible;
  late _RefreshingLibrary library;
  int committed = 0;
  const destination = LibraryOrganizationAlbum("destination", "Destination");

  setUp(() {
    batches = OrganizationBatchesFixture();
    service = OrganizationServiceFixture()..albumItems = [destination];
    visible = ValueNotifier(true);
    committed = 0;
    library = _RefreshingLibrary();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationBatchesProvider.overrideWithValue(batches),
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(() => library),
      ],
    );
  });
  tearDown(() {
    container.dispose();
    visible.dispose();
  });

  Future<void> host(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: ValueListenableBuilder<bool>(
              valueListenable: visible,
              builder: (context, show, _) => show
                  ? LibrarySelectionActions(
                      selection: GallerySelection.empty(
                        "query-42",
                      ).selectAll().toggle("excluded"),
                      query: const LibraryGalleryQuery(
                        rootId: "source",
                        searchText: "name",
                      ),
                      catalogRevision: BigInt.from(42),
                      enabled: true,
                      onCommitted: () => committed++,
                    )
                  : const SizedBox.shrink(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> favorite(WidgetTester tester, bool value) async {
    await tester.tap(find.byKey(const Key("library-selection-favorite")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(value ? "加入收藏夹" : "取消收藏"));
    await tester.pumpAndSettle();
  }

  Future<void> albums(WidgetTester tester) async {
    await tester.tap(find.byKey(const Key("library-selection-album")));
    await tester.pumpAndSettle();
  }

  Future<void> confirm(WidgetTester tester) async {
    await tester.pump();
    await tester.tap(find.byKey(const Key("library-album-confirm")));
    await tester.pumpAndSettle();
  }

  testWidgets(
    "both mixed-selection favorite commands stay transient and share undo",
    (tester) async {
      await host(tester);
      await favorite(tester, true);
      await favorite(tester, false);
      expect(
        batches.selections.every((selection) => selection.allMatching),
        isTrue,
      );
      expect(batches.selections.first.assetIds, ["excluded"]);
      expect(batches.selections.first.query.rootId, "source");
      expect(
        batches.batches.map(
          (batch) =>
              (batch.writes.single.action as LibraryBatchFavorite).favorite,
        ),
        [true, false],
      );
      expect(committed, 2);
      final notifications = container.read(ameNotificationControllerProvider);
      expect(notifications.history, isEmpty);
      expect(notifications.hasUnread, isFalse);
      final notice = container.read(libraryOrganizationOutcomeProvider)!;
      expect(notice.title, "已取消收藏");
      container
          .read(libraryOrganizationActionsProvider.notifier)
          .handleNotification(notice);
      await tester.pumpAndSettle();
      expect(
        service.undos.single.originalId,
        batches.batches.last.writes.single.id,
      );
      expect(batches.batches.every((batch) => batch.closes == 1), isTrue);
    },
  );

  testWidgets("mixed albums use the shared picker and remove whole groups", (
    tester,
  ) async {
    final batch = OrganizationBatchFixture()
      ..preparing = Completer<LibraryBatchSummary>();
    batches.create = () => batch;
    await host(tester);
    await tester.tap(find.byKey(const Key("library-selection-album")));
    await tester.pump();
    expect(find.textContaining("正在准备"), findsOneWidget);
    batch.preparing!.complete(batch.summary);
    await tester.pumpAndSettle();
    expect(find.textContaining("正在准备"), findsNothing);
    expect(find.text("移至相册"), findsOneWidget);
    expect(batches.batches.single.writes, isEmpty);
    await tester.tap(find.text("移出相册"));
    await tester.pumpAndSettle();
    expect(
      (batches.batches.single.writes.single.action as LibraryBatchAlbum)
          .albumId,
      isNull,
    );
    expect(committed, 1);
    expect(find.byKey(const Key("library-album-picker")), findsNothing);
    expect(batches.batches.single.closes, 1);
  });

  testWidgets("conflicts require a choice even with an automatic default", (
    tester,
  ) async {
    batches.create = () => OrganizationBatchFixture()
      ..summary = const LibraryBatchSummary(
        groups: 3,
        favorites: 1,
        assigned: 1,
        conflicts: 1,
      );
    await container
        .read(amePreferencesControllerProvider.notifier)
        .update(
          const AmePreferences(
            albums: AlbumPreferences(
              askBeforeAdding: false,
              defaultAlbumId: "destination",
            ),
          ),
        );
    await host(tester);
    await albums(tester);
    expect(find.text("所选图片中有相册归属冲突，请选择一个相册。"), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key("library-album-confirm")))
          .onPressed,
      isNull,
    );
    expect(batches.batches.single.writes, isEmpty);
    await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
    await confirm(tester);
    expect(
      (batches.batches.single.writes.single.action as LibraryBatchAlbum)
          .albumId,
      "destination",
    );
  });

  testWidgets(
    "create-in-picker retains the selection after its launching toolbar disappears",
    (tester) async {
      await host(tester);
      await albums(tester);
      library.afterRefresh = () => visible.value = false;
      await tester.tap(find.byKey(const Key("library-album-create")));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key("library-album-name")),
        "New album",
      );
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-selection-album")), findsNothing);
      expect(find.text("已选择：New album"), findsOneWidget);
      expect(batches.batches.single.refreshes, 1);
      expect(batches.batches.single.closes, 0);
      await confirm(tester);
      expect(batches.batches.single.writes, hasLength(1));
      expect(batches.batches.single.closes, 1);
      expect(committed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a valid automatic default applies the whole selection without opening a picker",
    (tester) async {
      await container
          .read(amePreferencesControllerProvider.notifier)
          .update(
            const AmePreferences(
              albums: AlbumPreferences(
                askBeforeAdding: false,
                defaultAlbumId: "destination",
              ),
            ),
          );
      await host(tester);
      await albums(tester);
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(
        (batches.batches.single.writes.single.action as LibraryBatchAlbum)
            .albumId,
        "destination",
      );
      expect(batches.batches.single.closes, 1);
      expect(committed, 1);
    },
  );

  testWidgets("unknown album saves retry the same operation in the same form", (
    tester,
  ) async {
    await host(tester);
    await albums(tester);
    final batch = batches.batches.single;
    batch.saving = (_) async => throw StateError("acknowledgement lost");
    await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
    await confirm(tester);
    expect(find.text("尚未确认保存结果，请重试。"), findsOneWidget);
    expect(committed, 0);
    batch.saving = null;
    await confirm(tester);
    expect(batch.writes.map((write) => write.id).toSet(), hasLength(1));
    expect(batch.preparations, 1);
    expect(committed, 1);
    expect(container.read(ameNotificationControllerProvider).history, isEmpty);
  });

  testWidgets(
    "cancel during preparation retires a late result without a notification",
    (tester) async {
      final batch = OrganizationBatchFixture()
        ..preparing = Completer<LibraryBatchSummary>();
      batches.create = () => batch;
      await host(tester);
      await tester.tap(find.byKey(const Key("library-selection-favorite")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("加入收藏夹"));
      await tester.pump();
      visible.value = false;
      await tester.pump();
      batch.preparing!.complete(batch.summary);
      await tester.pumpAndSettle();
      expect(batch.writes, isEmpty);
      expect(batch.closes, 1);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(committed, 0);
    },
  );

  testWidgets(
    "cancel during a batch save waits for cancellation before closing",
    (tester) async {
      final saving = Completer<LibraryOrganizationReceipt>();
      await host(tester);
      await albums(tester);
      final batch = batches.batches.single;
      batch.saving = (_) => saving.future;
      batch.onClose = () => saving.completeError(
        const LibraryOrganizationFailure(
          "organization_selection_cancelled",
          "cancelled",
        ),
      );
      await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pump();
      expect(find.text("正在保存"), findsOneWidget);
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(batch.closes, 1);
      expect(committed, 0);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
    },
  );

  testWidgets(
    "a commit that wins cancellation still publishes its receipt and undo",
    (tester) async {
      final saving = Completer<LibraryOrganizationReceipt>();
      await host(tester);
      await albums(tester);
      final batch = batches.batches.single;
      batch.saving = (_) => saving.future;
      batch.onClose = () =>
          saving.complete(batch.receipt(batch.writes.single.id));
      await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pump();
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(committed, 1);
      expect(
        container.read(libraryOrganizationOutcomeProvider)?.actionId,
        "organization.undo",
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
    },
  );
}

class _RefreshingLibrary extends OrganizationLibraryFixture {
  VoidCallback? afterRefresh;
  @override
  Future<bool> refreshCurrentQuery() async {
    final result = await super.refreshCurrentQuery();
    afterRefresh?.call();
    return result;
  }
}
