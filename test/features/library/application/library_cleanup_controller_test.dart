import "package:cedarflake_ame/features/library/application/library_cleanup_controller.dart";
import "package:cedarflake_ame/features/library/application/library_cleanup_state.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_cleanup.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_cleanup.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_cleanup_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late CleanupServiceFixture service;
  late ProviderContainer container;
  late LibraryCleanupController controller;
  setUp(() {
    service = CleanupServiceFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationCleanupProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
    controller = container.read(libraryCleanupControllerProvider.notifier);
  });
  tearDown(() => container.dispose());
  LibraryCleanupState state() =>
      container.read(libraryCleanupControllerProvider);
  Future<LibraryCleanupReady> inspect() async {
    final pending = controller.inspect(90);
    service.tasks.last.events.add(LibraryCleanupInspected(cleanupInspection()));
    await pending;
    return state() as LibraryCleanupReady;
  }

  test(
    "cancellation retains worker authority until its terminal result",
    () async {
      final pending = controller.inspect(90);
      controller.cancel();
      await controller.inspect(null);
      expect(service.tasks, hasLength(1));
      expect((state() as LibraryCleanupWorking).stopping, isTrue);
      expect(service.tasks.single.disposals, 0);
      service.tasks.single.events.add(const LibraryCleanupCancelled());
      await pending;
      expect((state() as LibraryCleanupFinished).cancelled, isTrue);
      expect(service.tasks.single.disposals, 1);
    },
  );

  test("superseded confirmation cannot clear a different inspection", () async {
    final old = await inspect();
    final current = await inspect();
    await controller.apply(old);
    expect(service.tasks, hasLength(2));
    expect(state(), same(current));
  });

  test(
    "unknown cleanup retry retains operation identity and accepts committed cancellation race",
    () async {
      final ready = await inspect();
      final first = controller.apply(ready);
      final original = service.tasks.last;
      original.events.addError(
        const LibraryCleanupFailure("write_unknown", "connection ended"),
      );
      await first;
      expect(state(), isA<LibraryCleanupFailed>());
      final retry = controller.retry();
      final repeated = service.tasks.last;
      expect(repeated.operationId, original.operationId);
      controller.cancel();
      repeated.events.add(LibraryCleanupCompleted(repeated.operationId!, 4));
      await retry;
      expect((state() as LibraryCleanupFinished).removedRecords, 4);
      expect((state() as LibraryCleanupFinished).cancelled, isFalse);
    },
  );

  test(
    "changed records require new confirmation and missing results remain retryable",
    () async {
      final ready = await inspect();
      final pending = controller.apply(ready);
      service.tasks.last.events.addError(
        const LibraryCleanupFailure("organization_cleanup_changed", "changed"),
      );
      await pending;
      final retry = controller.retry();
      expect(service.tasks.last.operationId, isNull);
      service.tasks.last.events.add(
        LibraryCleanupInspected(cleanupInspection()),
      );
      await retry;
      expect(state(), isA<LibraryCleanupReady>());
      final next = controller.apply(state() as LibraryCleanupReady);
      await service.tasks.last.events.close();
      await next;
      expect(
        (state() as LibraryCleanupFailed).failure.code,
        "cleanup_result_missing",
      );
    },
  );
}
