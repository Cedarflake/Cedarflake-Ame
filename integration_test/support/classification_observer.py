import json
import pathlib
import re
import sqlite3
import sys


def observe(storage):
  root = pathlib.Path(storage).resolve(strict=True)
  expected_parent = pathlib.Path(__file__).resolve().parents[2] / "build"
  if root.parent != expected_parent or not re.fullmatch(
    r"integration-storage-[a-f0-9]{32}", root.name
  ):
    raise ValueError("Classification observation requires isolated integration storage")
  catalog = root / "catalog" / "ame.sqlite3"
  with sqlite3.connect(catalog.as_uri() + "?mode=ro", uri=True, timeout=2) as db:
    db.execute("PRAGMA query_only = ON")
    db.execute("BEGIN")
    mode, run_id, configuration = db.execute(
      "SELECT mode, run_id, configuration_revision "
      "FROM classification_dispatch_state JOIN classification_state USING(singleton)"
    ).fetchone()
    predictions = dict(db.execute(
      "SELECT run_id, count(*) FROM classification_predictions GROUP BY run_id LIMIT 32"
    ))
    work = dict(db.execute(
      "SELECT phase, count(*) FROM classification_work_items "
      "WHERE configuration_revision = ? GROUP BY phase", (configuration,)
    ))
    issues = db.execute(
      "SELECT DISTINCT issue_code FROM classification_work_items "
      "WHERE configuration_revision = ? AND issue_code IS NOT NULL LIMIT 8",
      (configuration,),
    ).fetchall()
  features = {}
  cache = root / "catalog" / "classification-features" / "classification-features.sqlite3"
  if cache.exists():
    with sqlite3.connect(cache.as_uri() + "?mode=ro", uri=True, timeout=2) as db:
      db.execute("PRAGMA query_only = ON")
      features = dict(db.execute("SELECT hex(key), hex(checksum) FROM features LIMIT 1025"))
    if len(features) > 1024:
      raise ValueError("The client fixture exceeded its bounded feature window")
  return {
    "mode": mode,
    "run": run_id,
    "predictions": predictions,
    "work": work,
    "issues": [row[0] for row in issues],
    "features": features,
  }


if __name__ == "__main__":
  print(json.dumps(observe(sys.argv[1]), separators=(",", ":")))
