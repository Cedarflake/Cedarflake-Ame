import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/ame_preferences.dart";
import "ports/ame_preference_store.dart";

final initialAmePreferencesProvider = Provider<AmePreferences>(
  (ref) => const AmePreferences(),
);

final amePreferenceStoreProvider = Provider<AmePreferenceStore>(
  (ref) => throw StateError(
    "AmePreferenceStore must be bound during application startup",
  ),
);

final amePreferencesControllerProvider =
    NotifierProvider<AmePreferencesController, AmePreferences>(
      AmePreferencesController.new,
    );

class AmePreferencesController extends Notifier<AmePreferences> {
  @override
  AmePreferences build() => ref.watch(initialAmePreferencesProvider);

  Future<void> update(AmePreferences preferences) async {
    final previous = state;
    state = preferences;
    try {
      await ref
          .read(amePreferenceStoreProvider)
          .saveAmePreferences(preferences);
    } on Object {
      if (identical(state, preferences)) {
        state = previous;
      }
      rethrow;
    }
  }
}
