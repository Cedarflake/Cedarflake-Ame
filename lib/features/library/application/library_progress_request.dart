import "../domain/library_organization.dart";
import "../domain/library_review.dart";
import "library_organization.dart";
import "library_organization_requests.dart";
import "library_review_requests.dart";

class LibraryProgressRequest extends LibraryOrganizationRequest {
  LibraryProgressRequest(
    this.target,
    this.action, {
    this.errorSurface = LibraryOrganizationErrorSurface.notice,
  });

  final LibraryOrganizationTarget target;
  final LibraryProgressAction action;
  @override
  final LibraryOrganizationErrorSurface errorSurface;
  LibraryOrganizationPrepared? _prepared;

  @override
  String get successMessage => libraryProgressMessage(action);

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final prepared = _prepared ??= await service.prepare([target]);
    requireActive();
    return service.progress(id, prepared, action);
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _prepared = null;
  }
}

class LibraryProgressSession {
  LibraryProgressSession(this.create);

  final LibraryOrganizationRequest Function(LibraryProgressAction) create;
  LibraryOrganizationRequest? _attempt;
  LibraryProgressAction? _action;

  LibraryOrganizationRequest request(LibraryProgressAction action) {
    if (_attempt == null || action != _action) {
      _attempt?.cancel();
      _action = action;
      _attempt = create(action);
    }
    return _attempt!;
  }

  void dispose() => _attempt?.cancel();
}
