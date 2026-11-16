import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

final ameBuildIdentityProvider = Provider<AmeBuildIdentity>(
  (ref) => const AmeBuildIdentity.current(),
);

class AmeBuildIdentity {
  const AmeBuildIdentity({required this.name, required this.number});

  const AmeBuildIdentity.current()
    : name = appBuildName,
      number = appBuildNumber;

  final String? name;
  final String? number;

  String? get version => switch ((name, number)) {
    (null, _) => null,
    (final name?, null) => name,
    (final name?, final number?) => "$name+$number",
  };

  String get applicationLabel =>
      version == null ? "Cedarflake Ame" : "Cedarflake Ame $version";

  String startupDiagnostics(Object error) => "$applicationLabel\n$error";
}
