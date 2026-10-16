import "package:flutter_riverpod/flutter_riverpod.dart";

enum LibraryWorkspace { gallery, duplicates, settings }

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
