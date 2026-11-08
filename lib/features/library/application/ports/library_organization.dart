import "../../domain/library_organization.dart";
import "../../domain/library_organization_history.dart";
import "../../domain/library_review.dart";

abstract interface class LibraryOrganization {
  Future<LibraryOrganizationHistory> history();
  Future<LibraryOrganizationHistoryReceipt> stepHistory(
    String operationId,
    LibraryOrganizationHistoryDirection direction,
    String originalId,
    BigInt revision,
  );
  Future<LibraryOrganizationInspection> inspect(
    List<LibraryOrganizationTarget> targets,
  );
  Future<LibraryOrganizationPrepared> prepare(
    List<LibraryOrganizationTarget> targets,
  );
  Future<LibraryOrganizationReceipt> favorite(
    String operationId,
    LibraryOrganizationPrepared prepared,
    bool favorite,
  );
  Future<BigInt> revision();
  Future<LibraryOrganizationReceipt> progress(
    String operationId,
    LibraryOrganizationPrepared prepared,
    LibraryProgressAction action,
  );
  Future<LibraryOrganizationAlbumPage> albums({String? after});
  Future<LibraryOrganizationAlbum?> findAlbum(String id);
  Future<LibraryOrganizationReceipt> createAlbum(
    String operationId,
    BigInt revision,
    String name, {
    LibraryAlbumIcon icon = LibraryAlbumIcon.album,
  });
  Future<LibraryOrganizationAlbum> changeAlbumIcon(
    String albumId,
    LibraryAlbumIcon icon,
  );
  Future<LibraryOrganizationReceipt> renameAlbum(
    String operationId,
    BigInt revision,
    String albumId,
    String name,
  );
  Future<LibraryOrganizationReceipt> deleteAlbum(
    String operationId,
    BigInt revision,
    String albumId,
  );
  Future<LibraryOrganizationReceipt> assignAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
    String albumId,
  );
  Future<LibraryOrganizationReceipt> removeAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
  );
  Future<LibraryOrganizationHistoryReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  );
}
