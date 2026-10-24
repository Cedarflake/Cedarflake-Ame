# ADR 0031: Stream portable organization archives from a committed snapshot

Status: Accepted for validation.

Date: 2026-10-24.

## Context

R4 requires portable recovery of durable decisions independently of source media and rebuildable
previews. Export must remain coherent while catalog publication and user commands continue. A
catalog revision or local admission ordinal has meaning only within its original catalog; neither
is a portable identity or authority to overwrite a newer decision.

## Decision

Use an Ame-owned UTF-8 JSON Lines format, identified by `ame-organization-archive` and an integer
format version. Version 2 contains a header, ordered records and a completion footer. Record kinds,
field order and scalar types are explicit adapter contracts, independent of SQLite schema discovery.
The header records an opaque archive identifier, creation time and source schema version for
provenance. The footer contains record counts and a BLAKE3 digest of all preceding bytes, including
line endings. This detects incomplete or accidentally altered files; it is not authentication.

Read and write one record at a time with a 1 MiB encoded-line limit. Reject unsupported versions,
unknown kinds, incompatible fields, oversized lines, trailing data, missing completion and digest
or count mismatches. Format validation does not authorize restoration: relational integrity,
association evidence, revision reconciliation and atomic application remain separate restore checks.
Never accept SQL, scripts or serialized engine objects from an archive.

Export includes album definitions and retirements, lineage decisions, group mappings, bindings,
source ownership, conflicts, operation/undo provenance, the committed bookmark and its scope,
publication-order admissions and R3 representative/ignore decisions. Catalog-local source generations,
admission ordinals and location identifiers are namespaced hints within this archive. Include their
stored association evidence for subsequent reconciliation; they cannot identify a target-catalog row
by themselves. Active review sessions and their save authority are not restored. Source files,
previews, models and unrelated derived catalog records are excluded.

Take a committed SQLite online snapshot in owned derived storage using bounded page steps, then
release the live catalog copy before streaming records. Export reads no source media. The temporary
snapshot can be larger than the resulting archive; expose copy progress, support cancellation and
retain a bounded copy deadline. This cost occurs only for an explicit export, never on startup.
Write into an exclusively owned temporary file beside the selected destination, flush and validate
it, then publish without replacing an existing file. Cancellation and failures retire owned scratch
files and leave current decisions and existing destination files intact.
One application archive operation owns admission at a time; failure or cancellation releases it.
The destination must be outside configured source roots, catalog/settings directories and preview
storage, including their resolved aliases.

The application owns destination admission and use-case sequencing. A narrow repository port owns
the export request and progress; the SQLite adapter owns snapshot extraction, while its archive
codec owns framing and format validation. Only typed progress and a scalar receipt cross the bridge.

### Restoration admission

Inspection streams decoded records into a separate, exclusively owned SQLite staging file. Fixed
tables enforce scalar, uniqueness and relationship contracts; archive text is bound as data,
never executed as SQL. Version 1 is converted only after its original bytes and footer are verified.
The staging connection uses a bounded page cache and disk temporary storage.
Only this disposable, private database disables its journal and synchronous flushes. Live catalog
transactions are not held while reading or validating the backup, and no inspected records are
published to the current catalog.

Completion requires both framing/digest verification and relational validation: referenced records,
direct canonical groups, paired undo targets, retirement constraints, conflict state, bookmark scope
and revision ceilings must agree. Deliberately retained undo targets may reference a removed lineage;
they remain evidence for rejecting partial undo, not corruption to discard. Counts describe canonical
groups and active albums, not provenance aliases or retired definitions.

On Windows, hold the input read-only without write/delete sharing for the complete read, open with
no-recall intent, and reject offline/recall attributes before reading bytes. A resident reparse file
remains admissible. Stream cancellation retires staging; parsing and relationship checks provide
checkpoints. Inspection returns the archive identifier and verified checksum with bounded scalar
counts. Applying a backup uses the reconciliation/transaction boundary below;
inspection does not grant foreign location identifiers, revisions or session identifiers live authority.

### Association preview

After archive validation, compare stored bindings with one read-only snapshot of the current catalog.
An indexed, disk-backed temporary projection captures active published locations in windows of 256;
neither original files nor retained display-only fingerprints are read. Report matched and unmatched
canonical groups separately from uniquely identified preferred physical copies. Include the observed
catalog and organization revisions with the counts. The projection and read transaction expire at
the end of inspection; these numbers are explanatory, not write authority.

A compatible published `blake3-256-full-v1` fingerprint and byte length can identify content across
volumes and rebuilt catalogs. The existing Windows metadata-reuse boundary also accepts the same
volume-qualified native file identity, canonical ChangeTime token and byte length. Contradictory
compatible content evidence, including a completed fingerprint not yet published to the gallery,
disqualifies that metadata match. Missing or unsupported native evidence cannot be replaced by equal
paths, local asset identifiers or source-generation numbers.

Content equality alone does not identify an R3 preferred physical copy. Require its archived version
evidence to agree with the choice's content key and to identify exactly one current native file
location. Multiple hardlink locations in either the archive or current catalog remain unresolved;
one surviving hardlink does not establish which older alias was preferred. Version 2 appends a
positive native-copy count to each version-evidence row. It records the physical ambiguity when
that evidence was captured and survives later exports alongside new current evidence. Historical
and current evidence rows must not be counted together as simultaneous physical copies. Version 1
imports derive the count from their original native-identity rows in staging. Aggregate
repeated native versions before matching so many aliases or hardlinks do not multiply lookup work.
Cancellation checkpoints retire the entire private projection. Concurrent catalog writes can proceed
while all counts retain the same inspected snapshot. Application revalidates the archive
checksum and recomputes associations within its publication transaction.

### Atomic application and retained intent

A restore request identifies the inspected archive and checksum, observed catalog and organization
revisions, and a new operation identifier. Validate and stage the input before acquiring the existing
interactive writer admission. Reopen through the validated catalog session, attach only the owned
staging database, and recheck the request inside one immediate transaction. An identical operation
identifier returns its committed receipt without repeating writes, including after later decisions
or loss of the input file. A changed payload under that identifier is rejected.

Allocate a checked local revision interval for imported history; foreign revisions never become
current revisions directly. Keep the first verified archive identity/checksum immutable and retain
operation receipts. A later explicit application may recover records deliberately cleaned after an
earlier restore. Restored history keeps its original receipt authority, so restoring missing records
does not reauthorize an old undo against the newly restored values. Existing local album definitions,
retirements and decisions remain authoritative. Per-field restored markers distinguish imported
intent from subsequent local actions when independently retained lineages later merge.

Reserve distinct local source generations and fixed-length, namespaced asset/location identifiers
for archive evidence. Map source roots only through a unique compatible native-root identity;
unmatched ownership remains retained. Paths and foreign identifiers never authorize association.
Replay current catalog publication against this evidence within the restore transaction, retaining
unmatched groups and pending R3 choices for later compatible publication. A preferred physical copy
requires both original and current uniqueness; content equality alone can recover a group-level
ignore choice, but cannot choose its physical representative. A current R3 choice wins over an
imported one.

Retain archived queue order in a separate admission namespace, ahead of which current/new independent
work remains ordered. Link current versions to earlier positions only through compatible native or
published content evidence, using the same conflict guard as decision recovery. Keep a current
bookmark and active review session; import a bookmark only when none exists. After association,
refresh a present bookmark's ordering boundary from its logical group so restored ordering does
not redirect continuation to a different part of the queue. Keep its scope and handled-state marker.
Re-export retained
versions, admissions, root ownership and unresolved choices. Explicit source removal or cleanup
retires its applicable retained authority; automatic reimport cannot recreate it. Explicit backup
application remains a separate user action.

Receipt, history, field decisions, evidence, queue order and bookmark changes commit together.
Cancellation checkpoints before commit and any failed statement roll back the whole write. The
admitted connection owns temporary tables and its staging attachment and closes before the staging
file is retired, including on Windows. Archive input and original media remain read-only. No source
scan or new content hashing occurs in this operation.

## Dependencies and alternatives

Use serde 1.0.229 and serde_json 1.0.154 (MIT OR Apache-2.0), behind the archive adapter. Their typed
serialization avoids an application JSON parser and provides bounded per-record decoding. Retain
the already locked tempfile 3.27.0 (MIT OR Apache-2.0) as a production dependency for exclusive
scratch ownership and no-clobber publication. SQLite backup and BLAKE3 are existing dependencies.
Official provenance: [serde](https://crates.io/crates/serde/1.0.229),
[serde_json](https://crates.io/crates/serde_json/1.0.154), and
[tempfile](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html).

A whole-catalog backup preserves unrelated derived data and binds portable recovery to internal
schema. A single JSON array requires either whole-library buffering or a more involved streaming
parser. Copying live tables independently can mix different committed decisions. These alternatives
do not meet the selected boundary as directly.

## Validation and replacement

Cover populated and empty exports, Unicode, stored R3 and R4 evidence, concurrent live writes,
bounded records, cancellation, malformed/truncated files and preservation of an existing destination.
Verify the actual settings workflow before claiming user-facing export delivery. Integrated R4
verification measures temporary storage, elapsed time and memory at retained-library scale.

A later format uses another explicit version and converter; never reinterpret version 1 fields.
Replacing serialization or snapshot engines preserves the port and archived decisions. Restoration
must stage and validate before atomic publication, preserve unmatched intent and newer local choices,
and keep explicit backup application separate from automatic reindex/reassociation authority.
