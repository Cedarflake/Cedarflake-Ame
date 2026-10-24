import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_archive_controller.dart";
import "package:cedarflake_ame/features/library/application/library_archive_state.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_archive.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_archive.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_archive_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late ArchiveServiceFixture service;
  late OrganizationLibraryFixture library;
  late ProviderContainer container;
  late LibraryArchiveController controller;
  var disposed = false;

  setUp(() {
    disposed = false;
    service = ArchiveServiceFixture();
    library = OrganizationLibraryFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationArchivesProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(() => library),
      ],
    );
    controller = container.read(libraryArchiveControllerProvider.notifier);
  });
  tearDown(() {
    if (!disposed) container.dispose();
  });

  LibraryArchiveState state() =>
      container.read(libraryArchiveControllerProvider);
  Future<LibraryArchiveReady> inspect({int revision = 10}) async {
    final pending = controller.inspect("backup.ame-backup");
    service.tasks.last.events.add(
      LibraryArchiveInspected(archiveInspection(revision: revision)),
    );
    await pending;
    return state() as LibraryArchiveReady;
  }

  test(
    "cancellation waits for terminal retirement and prevents overlapping commands",
    () async {
      final pending = controller.export("new.ame-backup");
      final task = service.tasks.single;
      await controller.inspect("other.ame-backup");
      expect(service.tasks, hasLength(1));
      task.events.add(
        const LibraryArchiveProgress(LibraryArchivePhase.writing),
      );
      controller.cancel();
      expect(task.cancellations, 1);
      expect((state() as LibraryArchiveWorking).stopping, isTrue);
      expect(task.disposals, 0);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      task.events.add(const LibraryArchiveCancelled());
      await pending;
      expect((state() as LibraryArchiveFinished).cancelled, isTrue);
      expect(task.disposals, 1);
      expect(library.refreshes, 0);
    },
  );

  test("a superseded inspection cannot authorize restoration", () async {
    final old = await inspect();
    final current = await inspect(revision: 11);
    await controller.restore(old);
    expect(service.tasks, hasLength(2));
    final pending = controller.restore(current);
    final task = service.tasks.last;
    expect(task.inspection, same(current.inspection));
    task.events.add(LibraryArchiveRestored(task.operationId!));
    await pending;
    expect(library.refreshes, 1);
  });

  test(
    "an unconfirmed restore retries the same receipt identity and publishes only its success",
    () async {
      final ready = await inspect();
      final pending = controller.restore(ready);
      final first = service.tasks.last;
      await first.events.close();
      await pending;
      expect(state(), isA<LibraryArchiveFailed>());
      expect(library.refreshes, 0);
      final retry = controller.retry();
      final second = service.tasks.last;
      expect(second.operationId, first.operationId);
      expect(second.inspection, same(first.inspection));
      second.events.add(LibraryArchiveRestored(second.operationId!));
      await retry;
      expect(library.refreshes, 1);
      expect(
        container.read(ameNotificationControllerProvider).history.single.title,
        "整理记录已恢复",
      );
      expect(service.tasks.every((task) => task.disposals == 1), isTrue);
    },
  );

  test(
    "stale confirmation rechecks the backup and waits for new confirmation",
    () async {
      final ready = await inspect();
      final pending = controller.restore(ready);
      service.tasks.last.events.addError(
        const LibraryArchiveFailure(
          "organization_archive_inspection_changed",
          "stale",
        ),
      );
      await pending;
      final retry = controller.retry();
      expect(service.tasks.last.kind, "inspect");
      service.tasks.last.events.add(
        LibraryArchiveInspected(archiveInspection(revision: 12)),
      );
      await retry;
      expect(state(), isA<LibraryArchiveReady>());
      expect(
        service.tasks.where((task) => task.kind == "restore"),
        hasLength(1),
      );
      expect(library.refreshes, 0);
    },
  );

  test(
    "a commit that wins the cancellation race still refreshes the library",
    () async {
      final ready = await inspect();
      final pending = controller.restore(ready);
      final task = service.tasks.last;
      controller.cancel();
      task.events.add(LibraryArchiveRestored(task.operationId!));
      await pending;
      expect((state() as LibraryArchiveFinished).cancelled, isFalse);
      expect(library.refreshes, 1);
    },
  );

  test(
    "provider disposal cancels the owned task without publishing late completion",
    () async {
      final pending = controller.export("new.ame-backup");
      final task = service.tasks.single;
      container.dispose();
      disposed = true;
      expect(task.cancellations, 1);
      task.events.add(const LibraryArchiveExported());
      await pending;
      expect(task.disposals, 1);
      expect(library.refreshes, 0);
    },
  );

  test(
    "a different operation result cannot produce a success notice",
    () async {
      final pending = controller.export("new.ame-backup");
      service.tasks.last.events.add(const LibraryArchiveRestored("other"));
      await pending;
      expect(
        (state() as LibraryArchiveFailed).failure.code,
        "archive_result_unexpected",
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
    },
  );
}
