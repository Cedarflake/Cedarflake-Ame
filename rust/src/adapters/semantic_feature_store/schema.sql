CREATE TABLE semantic_store_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    feature_count INTEGER NOT NULL CHECK (feature_count >= 0),
    accounted_bytes INTEGER NOT NULL CHECK (accounted_bytes >= 0),
    budget_reached INTEGER NOT NULL CHECK (budget_reached IN (0, 1))
);
INSERT INTO semantic_store_state VALUES (1, 0, 0, 0, 0);

CREATE TABLE image_features (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    feature_key BLOB NOT NULL UNIQUE CHECK (length(feature_key) = 32),
    content_scheme TEXT NOT NULL,
    content_digest BLOB NOT NULL CHECK (length(content_digest) = 32),
    byte_length INTEGER NOT NULL CHECK (byte_length > 0),
    embedding_space BLOB NOT NULL CHECK (length(embedding_space) = 32),
    image_pipeline BLOB NOT NULL CHECK (length(image_pipeline) = 32),
    dimensions INTEGER NOT NULL CHECK (dimensions BETWEEN 1 AND 4096),
    vector BLOB NOT NULL CHECK (length(vector) = dimensions * 4),
    checksum BLOB NOT NULL CHECK (length(checksum) = 32)
);
CREATE INDEX image_features_model_sequence
    ON image_features(embedding_space, image_pipeline, dimensions, sequence);
