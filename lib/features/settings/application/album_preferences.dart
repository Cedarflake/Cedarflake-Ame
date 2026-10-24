class AlbumPreferences {
  const AlbumPreferences({this.askBeforeAdding = true, this.defaultAlbumId});

  final bool askBeforeAdding;
  final String? defaultAlbumId;

  factory AlbumPreferences.fromStoredValue(Object? value) {
    if (value is! Map<String, Object?>) return const AlbumPreferences();
    final id = value["defaultAlbumId"];
    return AlbumPreferences(
      askBeforeAdding: value["askBeforeAdding"] != false,
      defaultAlbumId: id is String && id.isNotEmpty ? id : null,
    );
  }

  Map<String, Object?> toStoredValue() => {
    "askBeforeAdding": askBeforeAdding,
    "defaultAlbumId": defaultAlbumId,
  };
}
