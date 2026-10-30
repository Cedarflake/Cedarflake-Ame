CREATE TABLE feature_cache_state (
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  clock INTEGER NOT NULL CHECK(clock >= 0),
  entry_count INTEGER NOT NULL CHECK(entry_count >= 0),
  accounted_bytes INTEGER NOT NULL CHECK(accounted_bytes >= 0)
) STRICT;
INSERT INTO feature_cache_state VALUES (1, 0, 0, 0);

CREATE TABLE features (
  key BLOB PRIMARY KEY CHECK(length(key) = 32),
  payload BLOB NOT NULL CHECK(length(payload) BETWEEN 4 AND 16384 AND length(payload) % 4 = 0),
  checksum BLOB NOT NULL CHECK(length(checksum) = 32),
  touched INTEGER NOT NULL CHECK(touched >= 0)
) STRICT;
CREATE INDEX feature_recency ON features(touched, key);

CREATE TRIGGER feature_insert AFTER INSERT ON features BEGIN
  UPDATE feature_cache_state
  SET entry_count = entry_count + 1, accounted_bytes = accounted_bytes + length(NEW.payload) + 256
  WHERE singleton = 1;
END;
CREATE TRIGGER feature_remove AFTER DELETE ON features BEGIN
  UPDATE feature_cache_state
  SET entry_count = entry_count - 1, accounted_bytes = accounted_bytes - length(OLD.payload) - 256
  WHERE singleton = 1;
END;
