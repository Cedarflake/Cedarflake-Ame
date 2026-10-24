import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/album_settings_section.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../library/support/library_organization_fixture.dart";

void main() {
  late _PreferenceStore store;
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  setUp(() {
    store = _PreferenceStore();
    service = OrganizationServiceFixture()
      ..albumItems = [const LibraryOrganizationAlbum("a", "Landscape")];
    container = ProviderContainer(
      overrides: [
        amePreferenceStoreProvider.overrideWithValue(store),
        libraryOrganizationProvider.overrideWithValue(service),
      ],
    );
  });
  tearDown(() => container.dispose());

  Future<void> host(WidgetTester tester) => tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: AlbumSettingsSection())),
    ),
  );

  testWidgets(
    "album settings save the switch and selected destination, including clearing",
    (tester) async {
      await host(tester);
      expect(
        tester
            .widget<Switch>(find.byKey(const Key("settings-album-ask")))
            .value,
        isTrue,
      );
      await tester.tap(find.byKey(const Key("settings-album-ask")));
      await tester.pumpAndSettle();
      expect(store.saved.last.albums.askBeforeAdding, isFalse);
      await tester.tap(find.byKey(const Key("settings-default-album")));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey("album-choice-a")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      expect(store.saved.last.albums.defaultAlbumId, "a");
      expect(find.text("Landscape"), findsOneWidget);
      expect(service.memberships, isEmpty);
      await tester.tap(find.byKey(const Key("settings-default-album")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("未设置"));
      await tester.pumpAndSettle();
      expect(store.saved.last.albums.defaultAlbumId, isNull);
      expect(store.saved.last.albums.askBeforeAdding, isFalse);
      final writes = store.saved.length;
      await tester.tap(find.byKey(const Key("settings-default-album")));
      await tester.pumpAndSettle();
      expect(find.text("取消"), findsNothing);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(store.saved, hasLength(writes));
    },
  );

  testWidgets(
    "failed preference save keeps the committed switch and allows retry",
    (tester) async {
      store.rejectSave = true;
      await host(tester);
      await tester.tap(find.byKey(const Key("settings-album-ask")));
      await tester.pumpAndSettle();
      expect(find.text("设置未能保存，请重试。"), findsOneWidget);
      expect(
        tester
            .widget<Switch>(find.byKey(const Key("settings-album-ask")))
            .value,
        isTrue,
      );
      store.rejectSave = false;
      await tester.tap(find.byKey(const Key("settings-album-ask")));
      await tester.pumpAndSettle();
      expect(find.text("设置未能保存，请重试。"), findsNothing);
      expect(store.saved.single.albums.askBeforeAdding, isFalse);
    },
  );
}

class _PreferenceStore implements AmePreferenceStore {
  final saved = <AmePreferences>[];
  bool rejectSave = false;

  @override
  Future<AmePreferences> loadAmePreferences() async =>
      saved.lastOrNull ?? const AmePreferences();

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    if (rejectSave) throw StateError("storage unavailable");
    saved.add(preferences);
  }
}
