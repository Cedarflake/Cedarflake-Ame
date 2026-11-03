enum LibraryClassificationMode { disabled, automatic, paused, stopped }

enum LibraryClassificationConfigurationAction { save, enable }

enum LibraryClassificationControl { pause, resume, disable, stop }

class LibraryClassificationCategory {
  const LibraryClassificationCategory({
    required this.id,
    required this.label,
    required this.descriptions,
  });
  final String id;
  final String label;
  final List<String> descriptions;
}

class LibraryClassificationSettings {
  const LibraryClassificationSettings({
    required this.revision,
    required this.dispatchRevision,
    required this.mode,
    required this.categories,
    required this.runId,
    required this.installationSelected,
  });
  final BigInt revision;
  final BigInt dispatchRevision;
  final LibraryClassificationMode mode;
  final List<LibraryClassificationCategory> categories;
  final String? runId;
  final bool installationSelected;
}
