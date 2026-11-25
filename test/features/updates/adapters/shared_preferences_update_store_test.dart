import "package:cedarflake_ame/features/updates/adapters/platform/shared_preferences_update_store.dart";
import "package:cedarflake_ame/features/updates/domain/application_update.dart";
import "package:flutter_test/flutter_test.dart";
import "package:shared_preferences/shared_preferences.dart";
import "package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart";
import "package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart";

void main() {
  test(
    "an unreadable preference store cannot silently enable automatic checks",
    () async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.empty();
      final store = SharedPreferencesUpdateStore(
        preferences: _UnreadablePreferences(),
      );
      await expectLater(
        store.load(defaultChannel: ApplicationUpdateChannel.stable),
        throwsA(
          isA<ApplicationUpdateFailure>().having(
            (failure) => failure.code,
            "code",
            ApplicationUpdateFailureCode.storage,
          ),
        ),
      );
    },
  );
  test(
    "update choices survive reopening and malformed state uses channel defaults",
    () async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.empty();
      final preferences = SharedPreferencesAsync();
      final store = SharedPreferencesUpdateStore(preferences: preferences);
      for (final channel in ApplicationUpdateChannel.values) {
        for (final policy in ApplicationUpdatePolicy.values) {
          await store.save(
            ApplicationUpdatePreferences(channel: channel, policy: policy),
          );
          final reopened = await SharedPreferencesUpdateStore(
            preferences: preferences,
          ).load(defaultChannel: ApplicationUpdateChannel.stable);
          expect(reopened.channel, channel);
          expect(reopened.policy, policy);
        }
      }
      for (final bad in [
        '{',
        'null',
        '{"version":2}',
        '{"version":1,"channel":"unknown","policy":"unknown"}',
      ]) {
        await preferences.setString("ame.updates.v1", bad);
        final result = await store.load(
          defaultChannel: ApplicationUpdateChannel.preview,
        );
        expect(result.channel, ApplicationUpdateChannel.preview);
        expect(result.policy, ApplicationUpdatePolicy.check);
      }
    },
  );
}

class _UnreadablePreferences extends SharedPreferencesAsync {
  @override
  Future<String?> getString(String key) async =>
      throw StateError("Storage unavailable");
}
