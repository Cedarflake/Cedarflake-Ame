pub(super) const SQL: &str = "
CREATE TABLE archive_album (
  album_id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
  revision INTEGER NOT NULL CHECK(revision > 0),
  created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
  updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
) STRICT;
CREATE TABLE archive_record (
  record_id TEXT PRIMARY KEY NOT NULL, album_id TEXT REFERENCES archive_album(album_id),
  disposition TEXT NOT NULL CHECK(disposition IN ('pending', 'deferred', 'organized', 'ignored')),
  favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
  organization_revision INTEGER NOT NULL CHECK(organization_revision >= 0),
  favorite_revision INTEGER NOT NULL CHECK(favorite_revision >= 0),
  created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
  updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
  unassociated_since_unix_ms INTEGER CHECK(unassociated_since_unix_ms >= 0),
  CHECK((disposition = 'organized') = (album_id IS NOT NULL))
) STRICT;
CREATE TABLE archive_binding (
  record_id TEXT NOT NULL REFERENCES archive_record(record_id), asset_id TEXT NOT NULL,
  source_generation INTEGER NOT NULL CHECK(source_generation > 0),
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  file_identity_scheme TEXT, file_identity_value TEXT, source_revision_token TEXT,
  fingerprint_scheme TEXT, digest TEXT CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
  CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
  CHECK((fingerprint_scheme IS NULL) = (digest IS NULL)),
  PRIMARY KEY(asset_id, source_generation)
) STRICT;
CREATE TABLE archive_source_owner (
  root_id TEXT NOT NULL REFERENCES archive_root_evidence(root_id),
  record_id TEXT NOT NULL REFERENCES archive_record(record_id), PRIMARY KEY(root_id, record_id)
) STRICT;
CREATE TABLE archive_group (
  record_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_record(record_id),
  group_id TEXT NOT NULL REFERENCES archive_record(record_id)
) STRICT;
CREATE TABLE archive_conflict (
  record_id TEXT NOT NULL REFERENCES archive_record(record_id),
  album_id TEXT NOT NULL REFERENCES archive_album(album_id),
  decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0), PRIMARY KEY(record_id, album_id)
) STRICT;
CREATE TABLE archive_operation (
  operation_id TEXT PRIMARY KEY NOT NULL,
  request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
  operation_kind TEXT NOT NULL CHECK(operation_kind IN ('favorite', 'organization', 'album', 'undo', 'restore', 'cleanup')),
  revision INTEGER NOT NULL CHECK(revision >= 0), changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
  created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0)
) STRICT;
CREATE TABLE archive_operation_change (
  operation_id TEXT NOT NULL REFERENCES archive_operation(operation_id),
  record_id TEXT NOT NULL REFERENCES archive_record(record_id),
  field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
  previous_album_id TEXT,
  previous_disposition TEXT CHECK(previous_disposition IN ('pending', 'deferred', 'organized', 'ignored')),
  previous_favorite INTEGER CHECK(previous_favorite IN (0, 1)),
  applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
  CHECK((field = 'favorite' AND previous_favorite IS NOT NULL AND previous_disposition IS NULL AND previous_album_id IS NULL)
     OR (field = 'organization' AND previous_favorite IS NULL AND previous_disposition IS NOT NULL
         AND (previous_disposition = 'organized') = (previous_album_id IS NOT NULL))),
  PRIMARY KEY(operation_id, record_id, field)
) STRICT;
CREATE TABLE archive_retired_album (
  album_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_album(album_id),
  revision INTEGER NOT NULL CHECK(revision > 0), retired_unix_ms INTEGER NOT NULL CHECK(retired_unix_ms >= 0)
) STRICT;
CREATE TABLE archive_album_change (
  operation_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_operation(operation_id),
  album_id TEXT NOT NULL REFERENCES archive_album(album_id), previous_name TEXT,
  previous_retired INTEGER NOT NULL CHECK(previous_retired IN (0, 1)),
  applied_revision INTEGER NOT NULL CHECK(applied_revision > 0)
) STRICT;
CREATE TABLE archive_operation_conflict (
  operation_id TEXT NOT NULL REFERENCES archive_operation(operation_id),
  record_id TEXT NOT NULL REFERENCES archive_record(record_id),
  album_id TEXT NOT NULL REFERENCES archive_album(album_id),
  decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0),
  PRIMARY KEY(operation_id, record_id, album_id)
) STRICT;
CREATE TABLE archive_operation_target (
  operation_id TEXT NOT NULL REFERENCES archive_operation(operation_id), record_id TEXT NOT NULL,
  field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')), PRIMARY KEY(operation_id, record_id, field)
) STRICT;
CREATE TABLE archive_review_scope (
  collection TEXT NOT NULL CHECK(collection IN ('library', 'favorites', 'album')),
  album_id TEXT, root_id TEXT, folder_relative_path TEXT,
  include_descendants INTEGER NOT NULL CHECK(include_descendants IN (0, 1)),
  search_text TEXT NOT NULL CHECK(length(search_text) <= 512),
  queue TEXT NOT NULL CHECK(queue IN ('pending', 'deferred', 'ignored')),
  CHECK((collection = 'album') = (album_id IS NOT NULL))
) STRICT;
CREATE UNIQUE INDEX archive_review_scope_singleton ON archive_review_scope((1));
CREATE TABLE archive_bookmark (
  record_id TEXT NOT NULL REFERENCES archive_record(record_id), admission_id INTEGER NOT NULL CHECK(admission_id > 0),
  root_id TEXT NOT NULL, location_id TEXT NOT NULL, after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)),
  updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
) STRICT;
CREATE UNIQUE INDEX archive_bookmark_singleton ON archive_bookmark((1));
CREATE TABLE archive_review_operation (
  operation_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_operation(operation_id), session_id TEXT NOT NULL,
  position_sequence INTEGER NOT NULL CHECK(position_sequence > 0),
  committed_sequence INTEGER NOT NULL CHECK(committed_sequence = position_sequence + 1)
) STRICT;
CREATE TABLE archive_admission (
  admission_id INTEGER PRIMARY KEY CHECK(admission_id > 0),
  source_generation INTEGER NOT NULL UNIQUE CHECK(source_generation > 0)
) STRICT;
CREATE TABLE archive_admission_source (
  admission_id INTEGER NOT NULL REFERENCES archive_admission(admission_id),
  root_id TEXT NOT NULL REFERENCES archive_root_evidence(root_id), PRIMARY KEY(admission_id, root_id)
) STRICT;
CREATE TABLE archive_root_evidence (
  root_id TEXT PRIMARY KEY NOT NULL, path TEXT NOT NULL, identity_scheme TEXT, identity_value TEXT,
  CHECK((identity_scheme IS NULL) = (identity_value IS NULL))
) STRICT;
CREATE TABLE archive_version_evidence (
  location_id TEXT PRIMARY KEY NOT NULL, asset_id TEXT NOT NULL,
  root_id TEXT NOT NULL REFERENCES archive_root_evidence(root_id),
  source_generation INTEGER NOT NULL CHECK(source_generation > 0),
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  file_identity_scheme TEXT, file_identity_value TEXT, source_revision_token TEXT,
  fingerprint_scheme TEXT, digest TEXT CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
  CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
  CHECK((fingerprint_scheme IS NULL) = (digest IS NULL))
) STRICT;
CREATE TABLE archive_duplicate_decision (
  scheme TEXT NOT NULL, digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0), preferred_location_id TEXT, preferred_asset_id TEXT,
  ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)), updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
  CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)), PRIMARY KEY(scheme, digest, byte_length)
) STRICT;
";
