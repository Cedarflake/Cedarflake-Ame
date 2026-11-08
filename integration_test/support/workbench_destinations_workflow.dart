import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyWorkbenchDestinationsWorkflow(
  WidgetTester tester,
  ProviderContainer container,
) async {
  final albums = await container.read(libraryOrganizationProvider).albums();
  expect(albums.albums, isNotEmpty);
  final album = albums.albums.first;
  final initial = container.read(amePreferencesControllerProvider).albums;
  expect(initial.defaultAlbumId, isNull);

  await _tap(tester, "library-sidebar-settings");
  await _tap(tester, "settings-default-album");
  await _waitFor(tester, () {
    return find
        .byKey(ValueKey("album-choice-${album.id}"))
        .evaluate()
        .isNotEmpty;
  });
  await _tap(tester, "album-choice-${album.id}");
  await _tap(tester, "library-album-confirm");
  await _waitFor(tester, () {
    return container
            .read(amePreferencesControllerProvider)
            .albums
            .defaultAlbumId ==
        album.id;
  });

  await _tap(tester, "settings-default-album");
  await tester.binding.handlePopRoute();
  await tester.pumpAndSettle();
  expect(
    container.read(amePreferencesControllerProvider).albums.defaultAlbumId,
    album.id,
  );

  await _tap(tester, "settings-default-album");
  await tester.tap(find.text("未设置"));
  await _waitFor(tester, () {
    return container
            .read(amePreferencesControllerProvider)
            .albums
            .defaultAlbumId ==
        null;
  });
  expect(
    container.read(amePreferencesControllerProvider).albums.askBeforeAdding,
    initial.askBeforeAdding,
  );
  await _tap(tester, "workbench-gallery");
  expect(tester.takeException(), isNull);
  debugPrint("AME_WORKBENCH_DESTINATIONS_NATIVE_WORKFLOW_PASSED");
}

Future<void> _tap(WidgetTester tester, String key) async {
  final target = find.byKey(Key(key));
  await tester.ensureVisible(target);
  await tester.pumpAndSettle();
  await tester.tap(target);
  await tester.pumpAndSettle();
}

Future<void> _waitFor(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 15));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Workbench destination did not settle");
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
  await tester.pumpAndSettle();
}
