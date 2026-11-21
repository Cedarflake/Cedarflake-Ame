class LibrarySettingsVisit {
  LibrarySettingsVisit(this._initialLocale);

  final String _initialLocale;
  bool _languageChanged = false;

  bool get languageChanged => _languageChanged;

  void observeLocale(String localeName) {
    if (localeName != _initialLocale) {
      _languageChanged = true;
    }
  }
}
