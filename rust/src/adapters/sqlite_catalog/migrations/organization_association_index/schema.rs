pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "organization_unassociated_records",
        "CREATE TABLE organization_unassociated_records (
           record_id TEXT PRIMARY KEY NOT NULL,
           group_id TEXT NOT NULL,
           FOREIGN KEY(record_id) REFERENCES organization_record_groups(record_id) ON DELETE CASCADE
         ) WITHOUT ROWID",
    ),
    (
        "index",
        "organization_unassociated_records_group",
        "CREATE INDEX organization_unassociated_records_group
         ON organization_unassociated_records(group_id, record_id)",
    ),
    (
        "trigger",
        "organization_unassociated_member_insert",
        "CREATE TRIGGER organization_unassociated_member_insert
         AFTER INSERT ON organization_record_groups
         BEGIN
           INSERT INTO organization_unassociated_records(record_id, group_id)
           SELECT NEW.record_id, NEW.group_id FROM organization_records
           WHERE record_id = NEW.record_id AND unassociated_since_unix_ms IS NOT NULL;
         END",
    ),
    (
        "trigger",
        "organization_unassociated_member_move",
        "CREATE TRIGGER organization_unassociated_member_move
         AFTER UPDATE OF group_id ON organization_record_groups
         WHEN NEW.group_id <> OLD.group_id
         BEGIN
           UPDATE organization_unassociated_records SET group_id = NEW.group_id
           WHERE record_id = NEW.record_id;
         END",
    ),
    (
        "trigger",
        "organization_unassociated_record_change",
        "CREATE TRIGGER organization_unassociated_record_change
         AFTER UPDATE OF unassociated_since_unix_ms ON organization_records
         WHEN NEW.unassociated_since_unix_ms IS NOT OLD.unassociated_since_unix_ms
         BEGIN
           DELETE FROM organization_unassociated_records
           WHERE record_id = NEW.record_id AND NEW.unassociated_since_unix_ms IS NULL;
           INSERT OR IGNORE INTO organization_unassociated_records(record_id, group_id)
           SELECT record_id, group_id FROM organization_record_groups
           WHERE record_id = NEW.record_id AND NEW.unassociated_since_unix_ms IS NOT NULL;
         END",
    ),
];
