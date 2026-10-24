import "package:flutter_riverpod/flutter_riverpod.dart";

enum LibraryWorkspace {
  gallery,
  albums,
  duplicates,
  settings;

  bool get isBrowsing => this == gallery || this == albums;
}

final libraryWorkbenchControllerProvider =
    NotifierProvider<LibraryWorkbenchController, LibraryWorkspace>(
      LibraryWorkbenchController.new,
    );

class LibraryWorkbenchController extends Notifier<LibraryWorkspace> {
  @override
  LibraryWorkspace build() => LibraryWorkspace.gallery;

  void select(LibraryWorkspace destination) {
    state = destination;
  }
}
