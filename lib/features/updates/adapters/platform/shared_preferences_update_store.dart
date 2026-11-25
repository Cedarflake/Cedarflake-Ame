import "dart:convert";

import "package:shared_preferences/shared_preferences.dart";

import "../../application/ports/application_update_service.dart";
import "../../domain/application_update.dart";

class SharedPreferencesUpdateStore implements ApplicationUpdatePreferenceStore {
  SharedPreferencesUpdateStore({SharedPreferencesAsync? preferences})
    : _preferences = preferences ?? SharedPreferencesAsync();

  static const _key = "ame.updates.v1";
  final SharedPreferencesAsync _preferences;
  Future<void> _writes = Future<void>.value();

  @override
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  }) async {
    final defaults = ApplicationUpdatePreferences(channel: defaultChannel);
    final String? encoded;
    try {
      encoded = await _preferences.getString(_key);
    } on Object catch (error) {
      throw ApplicationUpdateFailure(
        ApplicationUpdateFailureCode.storage,
        "Could not read application update preferences: $error",
      );
    }
    if (encoded == null) return defaults;
    final Object? value;
    try {
      value = jsonDecode(encoded);
    } on FormatException {
      return defaults;
    }
    if (value is! Map<String, Object?> || value["version"] != 1) {
      return defaults;
    }
    final channel = value["channel"];
    final policy = value["policy"];
    return ApplicationUpdatePreferences(
      channel: ApplicationUpdateChannel.values.firstWhere(
        (entry) => entry.name == channel,
        orElse: () => defaultChannel,
      ),
      policy: ApplicationUpdatePolicy.values.firstWhere(
        (entry) => entry.name == policy,
        orElse: () => ApplicationUpdatePolicy.check,
      ),
    );
  }

  @override
  Future<void> save(ApplicationUpdatePreferences preferences) {
    final write = _writes.then(
      (_) => _preferences.setString(
        _key,
        jsonEncode({
          "version": 1,
          "channel": preferences.channel.name,
          "policy": preferences.policy.name,
        }),
      ),
    );
    _writes = write.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return write;
  }
}
