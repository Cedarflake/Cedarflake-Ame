enum LibraryQueryUpdateOutcome { applied, busy, superseded, failed }

typedef LibraryQueryAttempt = Future<LibraryQueryUpdateOutcome> Function();
