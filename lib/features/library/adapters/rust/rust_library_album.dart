import "../../../../src/rust/domain/organization.dart" as dto;
import "../../domain/library_organization.dart";

LibraryOrganizationAlbum libraryAlbum(dto.OrganizationAlbum value) =>
    LibraryOrganizationAlbum(
      value.albumId,
      value.name,
      icon: switch (value.icon) {
        dto.OrganizationAlbumIcon.album => LibraryAlbumIcon.album,
        dto.OrganizationAlbumIcon.travel => LibraryAlbumIcon.travel,
        dto.OrganizationAlbumIcon.nature => LibraryAlbumIcon.nature,
        dto.OrganizationAlbumIcon.pets => LibraryAlbumIcon.pets,
        dto.OrganizationAlbumIcon.people => LibraryAlbumIcon.people,
        dto.OrganizationAlbumIcon.food => LibraryAlbumIcon.food,
        dto.OrganizationAlbumIcon.art => LibraryAlbumIcon.art,
        dto.OrganizationAlbumIcon.work => LibraryAlbumIcon.work,
        dto.OrganizationAlbumIcon.study => LibraryAlbumIcon.study,
        dto.OrganizationAlbumIcon.games => LibraryAlbumIcon.games,
        dto.OrganizationAlbumIcon.music => LibraryAlbumIcon.music,
        dto.OrganizationAlbumIcon.memories => LibraryAlbumIcon.memories,
      },
    );

dto.OrganizationAlbumIcon nativeAlbumIcon(LibraryAlbumIcon icon) =>
    switch (icon) {
      LibraryAlbumIcon.album => dto.OrganizationAlbumIcon.album,
      LibraryAlbumIcon.travel => dto.OrganizationAlbumIcon.travel,
      LibraryAlbumIcon.nature => dto.OrganizationAlbumIcon.nature,
      LibraryAlbumIcon.pets => dto.OrganizationAlbumIcon.pets,
      LibraryAlbumIcon.people => dto.OrganizationAlbumIcon.people,
      LibraryAlbumIcon.food => dto.OrganizationAlbumIcon.food,
      LibraryAlbumIcon.art => dto.OrganizationAlbumIcon.art,
      LibraryAlbumIcon.work => dto.OrganizationAlbumIcon.work,
      LibraryAlbumIcon.study => dto.OrganizationAlbumIcon.study,
      LibraryAlbumIcon.games => dto.OrganizationAlbumIcon.games,
      LibraryAlbumIcon.music => dto.OrganizationAlbumIcon.music,
      LibraryAlbumIcon.memories => dto.OrganizationAlbumIcon.memories,
    };
