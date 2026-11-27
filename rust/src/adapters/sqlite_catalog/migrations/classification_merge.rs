pub(super) const EVIDENCE_TABLE: &str = "CREATE TABLE classification_maintenance_evidence (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
    run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
    evidence_revision TEXT NOT NULL CHECK(length(evidence_revision) = 64),
    payload TEXT NOT NULL CHECK(json_valid(payload)
        AND (json_extract(payload, '$.version') IS 1
            OR (json_extract(payload, '$.version') IS 2
                AND json_extract(payload, '$.kind') IS 'equivalent_category_retirement'
                AND json_type(payload, '$.retirements') IS 'array'
                AND json_array_length(payload, '$.retirements') BETWEEN 1 AND 64)
            OR (json_extract(payload, '$.version') IS 3
                AND json_extract(payload, '$.kind') IS 'purpose_category_merge'
                AND json_type(payload, '$.merge') IS 'object'
                AND json_type(payload, '$.merge.category') IS 'text'
                AND json_type(payload, '$.merge.purpose') IS 'text'))
        AND length(CAST(payload AS BLOB)) BETWEEN 1 AND 524288)
) STRICT, WITHOUT ROWID";
