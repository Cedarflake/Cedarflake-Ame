class LibrarySynchronizationRetry {
  LibrarySynchronizationRetry({required this.tolerance, required this.now});

  final Duration tolerance;
  final DateTime Function() now;
  DateTime? _startedAt;
  String? _code;
  int attempt = 0;

  bool admit(String code) {
    switch (code) {
      case "catalog_database_busy":
      case "catalog_database_locked":
      case "library_synchronization_start_in_progress":
      case "library_synchronization_poll_in_progress":
        break;
      default:
        return false;
    }
    final observedAt = now();
    if (_code != code) {
      _code = code;
      _startedAt = observedAt;
      attempt = 1;
    } else {
      attempt += 1;
    }
    return observedAt.difference(_startedAt ?? observedAt) < tolerance;
  }

  void clear() {
    _startedAt = null;
    _code = null;
    attempt = 0;
  }
}
