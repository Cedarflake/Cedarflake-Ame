import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_conflict_status_reader.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/presentation/library_conflict_notifications.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_viewer_action.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_picker.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_conflict_label.dart";
import "package:cedarflake_ame/features/settings/application/album_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

LibraryConflictStatus status(int revision, int groups) =>
    LibraryConflictStatus(revision: BigInt.from(revision), groups: groups);

void main() {
  for (final scale in [1.0, 2.0]) {
    testWidgets(
      "conflict pages remain usable at minimum desktop size and scale $scale",
      (tester) async {
        tester.view.physicalSize = const Size(800, 560);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final service = OrganizationServiceFixture()
          ..albumItems = List.generate(
            32,
            (index) =>
                LibraryOrganizationAlbum("album-$index", "Destination $index"),
          );
        final conflicts = _PagedConflicts();
        final container = ProviderContainer(
          overrides: [
            libraryOrganizationProvider.overrideWithValue(service),
            libraryOrganizationConflictsProvider.overrideWithValue(conflicts),
            libraryControllerProvider.overrideWith(
              OrganizationLibraryFixture.new,
            ),
          ],
        );
        addTearDown(container.dispose);
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: MaterialApp(
              builder: (context, child) => MediaQuery(
                data: MediaQuery.of(
                  context,
                ).copyWith(textScaler: TextScaler.linear(scale)),
                child: child!,
              ),
              home: Scaffold(
                body: Builder(
                  builder: (context) => TextButton(
                    onPressed: () => showLibraryAlbumPicker(
                      context: context,
                      actionLabel: "加入相册",
                      explanation: "这组图片的相册归属有冲突，请选择一个相册。",
                      conflictTarget: organizationTarget("one"),
                    ),
                    child: const Text("Open"),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.tap(find.text("Open"));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        await tester.ensureVisible(find.text("下一页"));
        await tester.tap(find.text("下一页"));
        await tester.pumpAndSettle();
        expect(find.text("Last conflict（已删除）"), findsOneWidget);
        expect(conflicts.requests, [null, "page-one"]);
        await tester.tap(find.text("上一页"));
        await tester.pumpAndSettle();
        expect(find.text("Conflict 0"), findsOneWidget);
        final destination = find.byKey(const ValueKey("album-choice-album-0"));
        await tester.ensureVisible(destination);
        await tester.tap(destination);
        await tester.pumpAndSettle();
        expect(tester.widget<CheckboxListTile>(destination).value, isTrue);
        await tester.binding.handlePopRoute();
        await tester.pumpAndSettle();
        expect(find.byKey(const Key("library-album-picker")), findsNothing);
        expect(service.memberships, isEmpty);
        expect(tester.takeException(), isNull);
      },
    );
  }

  test(
    "conflict notices coalesce and dismissal never resolves persisted work",
    () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifications = container.read(
        ameNotificationControllerProvider.notifier,
      );
      final conflicts = LibraryConflictNotifications(notifications);
      conflicts.accept(status(1, 1));
      final first = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(conflicts.admit(first), isTrue);
      notifications.dismiss(first.id);
      conflicts.accept(status(2, 2));
      conflicts.accept(status(3, 2));
      var state = container.read(ameNotificationControllerProvider);
      expect(state.history, hasLength(1));
      expect(state.pendingIds, isEmpty);
      expect(state.history.single.isActive, isTrue);
      expect(state.history.single.message, startsWith("2 组图片"));
      conflicts.accept(status(1, 0));
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isTrue,
      );
      conflicts.accept(status(4, 0));
      state = container.read(ameNotificationControllerProvider);
      expect(state.history.single.isActive, isFalse);
      expect(conflicts.admit(first), isFalse);
    },
  );

  test(
    "conflict status reads coalesce publications and reject older results",
    () async {
      final first = Completer<LibraryConflictStatus>();
      var reads = 0;
      final reader = LibraryConflictStatusReader(
        () async => ++reads == 1 ? first.future : status(21, 2),
      );
      final observed = <LibraryConflictStatus>[];
      final subscription = reader.changes.listen(observed.add);
      reader.refresh(BigInt.one);
      for (var revision = 2; revision <= 21; revision++) {
        reader.refresh(BigInt.from(revision));
      }
      expect(reads, 1);
      first.complete(status(1, 1));
      await Future<void>.delayed(Duration.zero);
      expect(reads, 2);
      expect(observed.single.groups, 2);
      reader.close();
      await subscription.cancel();
    },
  );

  test(
    "failed status reads can recover and late close cannot publish",
    () async {
      final held = Completer<LibraryConflictStatus>();
      var reads = 0;
      final reader = LibraryConflictStatusReader(() async {
        reads++;
        if (reads == 1) throw StateError("temporarily unavailable");
        return held.future;
      });
      final errors = <Object>[];
      final observed = <LibraryConflictStatus>[];
      final subscription = reader.changes.listen(
        observed.add,
        onError: errors.add,
      );
      reader.refresh(BigInt.one);
      await Future<void>.delayed(Duration.zero);
      expect(errors, hasLength(1));
      reader.refresh(BigInt.two);
      expect(reads, 2);
      reader.close();
      held.complete(status(2, 1));
      await Future<void>.delayed(Duration.zero);
      expect(observed, isEmpty);
      await subscription.cancel();
    },
  );

  testWidgets("conflict label survives notification dismissal", (tester) async {
    final container = ProviderContainer(
      overrides: [
        libraryConflictAnnotationsProvider.overrideWith((ref) async => {"one"}),
      ],
    );
    addTearDown(container.dispose);
    final notifications = container.read(
      ameNotificationControllerProvider.notifier,
    );
    LibraryConflictNotifications(notifications).accept(status(1, 1));
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: 180,
              height: 180,
              child: LibraryConflictLabel(locationId: "one"),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    notifications.dismiss(
      container.read(ameNotificationControllerProvider).history.single.id,
    );
    await tester.pumpAndSettle();
    expect(find.text("相册归属冲突"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "conflict picker keeps provenance through cancellation and failed saves",
    (tester) async {
      final service = OrganizationServiceFixture()
        ..albumItems = [
          const LibraryOrganizationAlbum("destination", "Destination"),
        ];
      var conflicted = true;
      service.inspectTarget = (_) async => LibraryOrganizationInspection(
        revision: BigInt.one,
        decisions: [
          LibraryOrganizationDecision(
            favorite: true,
            disposition: conflicted
                ? LibraryOrganizationDisposition.deferred
                : LibraryOrganizationDisposition.organized,
            hasConflict: conflicted,
          ),
        ],
      );
      final container = ProviderContainer(
        overrides: [
          libraryOrganizationProvider.overrideWithValue(service),
          libraryOrganizationConflictsProvider.overrideWithValue(_Conflicts()),
          libraryControllerProvider.overrideWith(
            OrganizationLibraryFixture.new,
          ),
        ],
      );
      addTearDown(container.dispose);
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
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(
              body: LibraryAlbumViewerAction(target: organizationTarget("one")),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("viewer-album-conflict")));
      await tester.pumpAndSettle();
      expect(find.text("风景（已删除）"), findsOneWidget);
      expect(find.text("Archive"), findsOneWidget);
      expect(service.memberships, isEmpty);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("viewer-album-conflict")), findsOneWidget);
      expect(service.memberships, isEmpty);
      await tester.tap(find.byKey(const Key("viewer-album-conflict")));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
      await tester.pump();
      service.saveMembership = (_) async =>
          throw StateError("save unavailable");
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(find.text("尚未确认保存结果，请重试。"), findsOneWidget);
      expect(find.text("Archive"), findsOneWidget);
      service.saveMembership = (id) async {
        conflicted = false;
        return service.receipt(id);
      };
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(service.memberships, hasLength(2));
      expect(service.memberships.last.id, service.memberships.first.id);
      expect(find.byKey(const Key("library-album-picker")), findsNothing);
      expect(find.byKey(const Key("viewer-album-conflict")), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}

class _Conflicts implements LibraryOrganizationConflicts {
  @override
  Future<LibraryConflictStatus> status() async =>
      LibraryConflictStatus(revision: BigInt.one, groups: 1);
  @override
  Future<Set<String>> annotations(
    BigInt revision,
    List<String> locationIds,
  ) async => {"one"};
  @override
  Future<LibraryConflictAlbums> albums(
    LibraryOrganizationTarget target, {
    String? after,
  }) async => LibraryConflictAlbums(
    revision: BigInt.one,
    albums: const [
      LibraryConflictAlbum(id: "retired", name: "风景", retired: true),
      LibraryConflictAlbum(id: "archive", name: "Archive", retired: false),
    ],
  );
}

class _PagedConflicts extends _Conflicts {
  final requests = <String?>[];

  @override
  Future<LibraryConflictAlbums> albums(
    LibraryOrganizationTarget target, {
    String? after,
  }) async {
    requests.add(after);
    return LibraryConflictAlbums(
      revision: BigInt.one,
      nextAlbumId: after == null ? "page-one" : null,
      albums: after == null
          ? List.generate(
              64,
              (index) => LibraryConflictAlbum(
                id: "conflict-$index",
                name: "Conflict $index",
                retired: false,
              ),
            )
          : const [
              LibraryConflictAlbum(
                id: "last",
                name: "Last conflict",
                retired: true,
              ),
            ],
    );
  }
}
