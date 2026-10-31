pub(super) const SQL: &str = "
CREATE TABLE archive_classification_category (
  category_id TEXT PRIMARY KEY NOT NULL CHECK(length(category_id) BETWEEN 1 AND 64),
  label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 80), revision INTEGER NOT NULL CHECK(revision >= 0)
) STRICT;
CREATE TABLE archive_classification_version (
  category_id TEXT NOT NULL REFERENCES archive_classification_category(category_id),
  semantic_revision TEXT NOT NULL CHECK(length(semantic_revision) = 64),
  descriptions TEXT NOT NULL CHECK(json_valid(descriptions) AND json_type(descriptions) = 'array'
    AND json_array_length(descriptions) BETWEEN 1 AND 8 AND length(CAST(descriptions AS BLOB)) <= 65536),
  PRIMARY KEY(category_id, semantic_revision)
) STRICT;
CREATE TABLE archive_classification_rules (
  rules_revision TEXT PRIMARY KEY NOT NULL CHECK(length(rules_revision) = 64)
) STRICT;
CREATE TABLE archive_classification_member (
  rules_revision TEXT NOT NULL REFERENCES archive_classification_rules(rules_revision),
  category_id TEXT NOT NULL, semantic_revision TEXT NOT NULL,
  PRIMARY KEY(rules_revision, category_id),
  FOREIGN KEY(category_id, semantic_revision) REFERENCES archive_classification_version(category_id, semantic_revision)
) STRICT;
CREATE TABLE archive_classification_decision (
  record_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_record(record_id),
  category_id TEXT REFERENCES archive_classification_category(category_id),
  choice TEXT CHECK(choice IS NULL OR (json_valid(choice) AND length(CAST(choice AS BLOB)) <= 4096)),
  choice_revision INTEGER NOT NULL CHECK(choice_revision >= 0),
  choice_restored INTEGER NOT NULL CHECK(choice_restored IN (0, 1)),
  deferred INTEGER NOT NULL CHECK(deferred IN (0, 1)),
  review_revision INTEGER NOT NULL CHECK(review_revision >= 0),
  review_restored INTEGER NOT NULL CHECK(review_restored IN (0, 1)),
  CHECK((category_id IS NULL) = (choice IS NULL)),
  CHECK(choice IS NULL OR choice_revision > 0), CHECK(deferred = 0 OR review_revision > 0)
) STRICT;
CREATE TABLE archive_classification_operation (
  operation_id TEXT PRIMARY KEY NOT NULL CHECK(length(operation_id) BETWEEN 1 AND 128),
  request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
  operation_kind TEXT NOT NULL CHECK(operation_kind IN ('configure', 'category', 'defer', 'undo', 'restore', 'continuation')),
  revision INTEGER NOT NULL CHECK(revision >= 0), changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
  created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0), restored INTEGER NOT NULL CHECK(restored IN (0, 1))
) STRICT;
CREATE TABLE archive_classification_change (
  operation_id TEXT NOT NULL REFERENCES archive_classification_operation(operation_id),
  record_id TEXT NOT NULL REFERENCES archive_record(record_id), field TEXT NOT NULL CHECK(field IN ('category', 'review')),
  previous_payload TEXT CHECK(previous_payload IS NULL OR (json_valid(previous_payload) AND length(CAST(previous_payload AS BLOB)) <= 4096)),
  previous_revision INTEGER NOT NULL CHECK(previous_revision >= 0),
  previous_restored INTEGER NOT NULL CHECK(previous_restored IN (0, 1)),
  applied_revision INTEGER NOT NULL CHECK(applied_revision > 0), PRIMARY KEY(operation_id, record_id, field),
  CHECK(field <> 'review' OR (previous_payload IS NOT NULL AND previous_payload IN ('true', 'false')))
) STRICT;
CREATE TABLE archive_classification_target (
  operation_id TEXT NOT NULL REFERENCES archive_classification_operation(operation_id),
  record_id TEXT NOT NULL, field TEXT NOT NULL CHECK(field IN ('category', 'review')),
  PRIMARY KEY(operation_id, record_id, field)
) STRICT;
CREATE TABLE archive_classification_bookmark (
  record_id TEXT NOT NULL REFERENCES archive_record(record_id),
  scope TEXT NOT NULL CHECK(json_valid(scope) AND length(CAST(scope AS BLOB)) <= 8192),
  admission_id INTEGER NOT NULL REFERENCES archive_admission(admission_id),
  root_id TEXT NOT NULL REFERENCES archive_root_evidence(root_id), location_id TEXT NOT NULL,
  primary_missing INTEGER NOT NULL CHECK(primary_missing IN (0, 1)),
  primary_text TEXT NOT NULL CHECK(length(primary_text) <= 32768), primary_number INTEGER NOT NULL,
  after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)), revision INTEGER NOT NULL CHECK(revision > 0)
) STRICT;
CREATE UNIQUE INDEX archive_classification_bookmark_singleton ON archive_classification_bookmark((1));
CREATE TABLE archive_classification_context (
  revision INTEGER NOT NULL CHECK(revision >= 0), active_rules TEXT REFERENCES archive_classification_rules(rules_revision)
) STRICT;
CREATE UNIQUE INDEX archive_classification_context_singleton ON archive_classification_context((1));
";
